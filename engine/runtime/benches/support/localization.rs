use std::collections::BTreeMap;

use nanika_localization::{ResourceSet, SupportedLocale, candidate_search_text};
use nanika_protocol::{Action, Candidate as Publication, CandidateKind, message};
use nanika_search::{Candidate, UsageKey, UsageMap, UsageStat};

pub const SIZES: [usize; 5] = [100, 1_000, 5_000, 10_000, 50_000];
pub const QUERIES: [(&str, &str); 6] = [
    ("empty", ""),
    ("english", "application 42"),
    ("chinese", "应用程序 42"),
    ("alias", "shortcut42"),
    ("pinyin", "yingyongchengxu42"),
    ("usage", "application")
];

pub fn resources(bilingual: bool) -> ResourceSet {
    let mut sources = BTreeMap::from([(
        SupportedLocale::English,
        vec![(
            "en.ftl".into(),
            "title = Application { $index }\nalias = shortcut{ $index }\nopen = Open".into()
        )]
    )]);
    if bilingual {
        sources.insert(
            SupportedLocale::SimplifiedChinese,
            vec![(
                "zh-Hans.ftl".into(),
                "title = 应用程序 { $index }\nalias = 快捷方式{ $index }\nopen = 打开".into()
            )]
        );
    }
    ResourceSet::new(SupportedLocale::English, sources, bilingual).unwrap()
}

/// Match the production candidate-search preparation boundary, excluding process transport,
/// package resource loading, protocol receipt validation and durable catalog storage.
pub fn candidates(count: usize, resources: Option<&ResourceSet>) -> Vec<Candidate> {
    (0..count)
        .map(|index| {
            let publication = publication(index, resources.is_some());
            let search = candidate_search_text(&publication, resources, "macos").unwrap();
            indexed_candidate(publication, search)
        })
        .collect()
}

/// Build exactly the same protocol payload for combined and phase measurements.
pub fn publication(index: usize, localized: bool) -> Publication {
    let (title, alias, action) = if localized {
        let args = [("index".into(), i64::try_from(index).unwrap().into())];
        (
            message("title", args.clone()),
            message("alias", args),
            message("open", [])
        )
    } else {
        (
            format!("Application {index}").into(),
            format!("shortcut{index}").into(),
            "Open".into()
        )
    };
    Publication {
        kind: CandidateKind::Action,
        entry_id: format!("entry-{index}"),
        title,
        subtitle: None,
        action_id: "open".into(),
        actions: vec![Action::primary("open", action)],
        aliases: vec![alias],
        icon: None
    }
}

/// Consume admitted fields through the production search-index constructor.
pub fn indexed_candidate(publication: Publication, search: Vec<String>) -> Candidate {
    Candidate::new(
        nanika_search::CandidateKind::Action,
        "benchmark",
        publication.entry_id,
        publication.title,
        publication.action_id,
        publication.actions,
        search
    )
}

pub fn usage() -> UsageMap {
    (0..100)
        .map(|index| {
            (
                UsageKey::new(
                    "benchmark",
                    &format!("entry-{index}"),
                    "open",
                    "application"
                ),
                UsageStat {
                    execution_count: 10,
                    last_executed_at: 1_000
                }
            )
        })
        .collect()
}
