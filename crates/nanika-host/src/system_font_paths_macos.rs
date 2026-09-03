use std::path::PathBuf;

pub(super) fn primary_candidates() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

pub(super) fn fallback_candidates() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}
