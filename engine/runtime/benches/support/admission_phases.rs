//! Coarse admission phases. Setup and final result destruction are outside each timed region.
//! Batched phases have different allocation lifetimes from interleaved admission; their
//! percentiles are diagnostic measurements and must not be summed or used as the admission gate.
use std::hint::black_box;
use std::time::Instant;

use nanika_localization::{ResourceSet, candidate_search_text};

use super::{_report, support};

pub fn measure(mode: &str, count: usize, resources: Option<&ResourceSet>) {
    let mut publication_samples = Vec::with_capacity(31);
    let mut formatting_samples = Vec::with_capacity(31);
    let mut indexing_samples = Vec::with_capacity(31);
    let expected = support::candidates(count, resources);
    for _ in 0..31 {
        let start = Instant::now();
        let publications: Vec<_> = (0..count)
            .map(|index| support::publication(index, resources.is_some()))
            .collect();
        publication_samples.push(start.elapsed());
        black_box(&publications);

        let start = Instant::now();
        let search: Vec<_> = publications
            .iter()
            .map(|publication| candidate_search_text(publication, resources, "macos").unwrap())
            .collect();
        formatting_samples.push(start.elapsed());
        black_box(&search);

        let start = Instant::now();
        let entries: Vec<_> = publications
            .into_iter()
            .zip(search)
            .map(|(publication, search)| support::indexed_candidate(publication, search))
            .collect();
        indexing_samples.push(start.elapsed());
        // Equality covers every retained candidate field, prepared spelling and romanized reading.
        assert_eq!(entries, expected);
        black_box(entries);
    }
    for (phase, samples) in [
        ("publication", &mut publication_samples),
        ("formatting", &mut formatting_samples),
        ("indexing", &mut indexing_samples)
    ] {
        _report("admission_phase", mode, count, phase, count, samples);
    }
}
