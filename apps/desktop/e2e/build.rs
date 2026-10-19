fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Tauri embeds this dependency in its own binary. Library consumers must
        // also request v6, otherwise Windows cannot resolve TaskDialogIndirect.
        println!("cargo:rustc-link-arg-bin=nanika-e2e=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=nanika-e2e=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
