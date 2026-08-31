use std::path::PathBuf;

pub(super) fn candidates() -> Vec<(PathBuf, &'static str)> {
    [
        (
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "Hiragino Sans GB",
        ),
        ("/System/Library/Fonts/STHeiti Medium.ttc", "Heiti SC"),
        (
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "Arial Unicode MS",
        ),
    ]
    .into_iter()
    .map(|(path, family)| (PathBuf::from(path), family))
    .collect()
}
