use pinyin::ToPinyin;

use crate::han_readings::READINGS;

/// A prepared name spelling, with the first letter of each Han syllable or Latin run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomanizedReading {
    pub full: String,
    pub initials: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomanizedMatch<'a> {
    Exact,
    Prefix(&'a str),
    Infix(&'a str),
}

impl RomanizedMatch<'_> {
    fn rank(self) -> u8 {
        match self {
            Self::Exact => 3,
            Self::Prefix(_) => 2,
            Self::Infix(_) => 1,
        }
    }
}

/// Find the strongest contiguous spelling match without changing the cached readings.
pub fn find_romanized_match<'a>(
    readings: &'a [RomanizedReading],
    query: &[RomanizedReading],
) -> Option<RomanizedMatch<'a>> {
    let mut best = None;
    for reading in readings {
        for spelling in query {
            if spelling.full.is_empty() {
                continue;
            }
            let Some(offset) = reading.full.find(&spelling.full) else {
                continue;
            };
            let matched = if offset == 0 && reading.full.len() == spelling.full.len() {
                RomanizedMatch::Exact
            } else if offset == 0 {
                RomanizedMatch::Prefix(&reading.full[spelling.full.len()..])
            } else {
                RomanizedMatch::Infix(&reading.full[..offset])
            };
            if best.is_none_or(|previous: RomanizedMatch<'_>| matched.rank() > previous.rank()) {
                best = Some(matched);
            }
        }
    }
    best
}

enum Part<'a> {
    Phrase(&'a str, &'static [(&'static str, &'static str)]),
    Syllable(&'static str),
    Literal(char),
}

/// Prepare a name once, before publishing its search aliases. Latin-only names need none.
pub fn romanized_readings(value: &str) -> Vec<RomanizedReading> {
    if !value
        .chars()
        .any(|character| character.to_pinyin().is_some())
    {
        return Vec::new();
    }
    let boundaries = value
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([value.len()])
        .collect::<Vec<_>>();
    let mut parts = Vec::new();
    let mut ambiguous = Vec::<&str>::new();
    let mut index = 0;
    while index + 1 < boundaries.len() {
        let character = value[boundaries[index]..boundaries[index + 1]]
            .chars()
            .next()
            .expect("character boundary has one character");
        if let Some(reading) = character.to_pinyin() {
            let mut phrase = None;
            for end in (index + 2..=(index + 10).min(boundaries.len() - 1)).rev() {
                let word = &value[boundaries[index]..boundaries[end]];
                let offset = READINGS.partition_point(|(key, _)| *key < word);
                if READINGS.get(offset).is_some_and(|(key, _)| *key == word) {
                    let last = if READINGS
                        .get(offset + 1)
                        .is_some_and(|(key, _)| *key == word)
                    {
                        offset + 2
                    } else {
                        offset + 1
                    };
                    phrase = Some((word, &READINGS[offset..last], end));
                    break;
                }
            }
            if let Some((word, readings, end)) = phrase {
                if readings.len() > 1 && !ambiguous.contains(&word) {
                    ambiguous.push(word);
                }
                parts.push(Part::Phrase(word, readings));
                index = end;
            } else {
                parts.push(Part::Syllable(reading.plain()));
                index += 1;
            }
        } else {
            parts.push(Part::Literal(character));
            index += 1;
        }
    }

    // The pinned dictionary has two ambiguous word keys. The same word takes a
    // consistent reading throughout a name, bounding complete variants at four.
    assert!(
        ambiguous.len() <= 2,
        "phrase variants exceed the indexed policy"
    );
    let mut result = Vec::with_capacity(1 << ambiguous.len());
    for selection in 0..(1 << ambiguous.len()) {
        let mut full = String::with_capacity(value.len());
        let mut initials = String::with_capacity(value.len());
        let mut latin_run = false;
        for part in &parts {
            match part {
                Part::Phrase(word, readings) => {
                    let choice = ambiguous
                        .iter()
                        .position(|key| key == word)
                        .map_or(0, |bit| (selection >> bit) & 1);
                    for syllable in readings[choice].1.split_whitespace() {
                        append_syllable(&mut full, &mut initials, syllable);
                    }
                    latin_run = false;
                }
                Part::Syllable(syllable) => {
                    append_syllable(&mut full, &mut initials, syllable);
                    latin_run = false;
                }
                Part::Literal(character) if character.is_ascii_alphanumeric() => {
                    let letter = character.to_ascii_lowercase();
                    full.push(letter);
                    if !latin_run {
                        initials.push(letter);
                    }
                    latin_run = true;
                }
                Part::Literal(character) if character.is_alphanumeric() => {
                    for letter in character.to_lowercase() {
                        full.push(letter);
                        initials.push(letter);
                    }
                    latin_run = false;
                }
                Part::Literal(_) => latin_run = false,
            }
        }
        let variant = RomanizedReading { full, initials };
        if !result.contains(&variant) {
            result.push(variant);
        }
    }
    result
}

/// Prepare mixed Han/Latin input once for comparison with cached spellings.
pub fn romanized_query(value: &str) -> Vec<RomanizedReading> {
    if !value
        .chars()
        .any(|character| character.is_ascii_alphanumeric())
    {
        return Vec::new();
    }
    romanized_readings(value)
}

fn append_syllable(full: &mut String, initials: &mut String, syllable: &str) {
    for letter in syllable.chars() {
        full.push(match letter {
            'ü' => 'v',
            'ê' => 'e',
            other => other,
        });
    }
    if let Some(first) = syllable.chars().next() {
        initials.push(first);
    }
}
