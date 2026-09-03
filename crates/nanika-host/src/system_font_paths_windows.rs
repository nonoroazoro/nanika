use std::path::PathBuf;

pub(super) fn primary_candidates() -> Vec<PathBuf> {
    let Some(windows_root) = std::env::var_os("WINDIR") else {
        return Vec::new();
    };
    let font_root = PathBuf::from(windows_root).join("Fonts");
    ["segoeui.ttf", "segoeuisl.ttf"]
        .into_iter()
        .map(|file| font_root.join(file))
        .collect()
}

pub(super) fn fallback_candidates() -> Vec<PathBuf> {
    let Some(windows_root) = std::env::var_os("WINDIR") else {
        return Vec::new();
    };
    let font_root = PathBuf::from(windows_root).join("Fonts");
    ["msyh.ttc", "msyhbd.ttc", "simsun.ttc"]
        .into_iter()
        .map(|file| font_root.join(file))
        .collect()
}
