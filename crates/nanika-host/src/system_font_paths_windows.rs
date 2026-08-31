use std::path::PathBuf;

pub(super) fn candidates() -> Vec<(PathBuf, &'static str)> {
    let Some(windows_root) = std::env::var_os("WINDIR") else {
        return Vec::new();
    };
    let font_root = PathBuf::from(windows_root).join("Fonts");
    [
        ("msyh.ttc", "Microsoft YaHei"),
        ("msyhbd.ttc", "Microsoft YaHei"),
        ("simsun.ttc", "SimSun"),
    ]
    .into_iter()
    .map(|(file, family)| (font_root.join(file), family))
    .collect()
}
