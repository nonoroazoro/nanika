#[cfg(target_os = "macos")]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Supply native bundle metadata inside the owned E2E session, independent of installed apps.
pub fn seed(
    directory: &Path,
    store: &nanika_config::ConfigStore
) -> Result<(), Box<dyn std::error::Error>> {
    let applications = directory.join("applications");
    _seed(&applications)?;
    store.save(
        store.extension_configuration_file("com.nanika.application"),
        &nanika_config::ExtensionConfigurationFile::new(
            [(
                "application.roots".into(),
                serde_json::json!([applications])
            )]
            .into()
        )
    )?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn _seed(applications: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(applications)?;
    // A real PE fixture exercises discovery without depending on installed applications.
    std::fs::copy(
        std::env::current_exe()?,
        applications.join("Nanika Reveal.exe")
    )?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn _seed(applications: &Path) -> Result<(), Box<dyn std::error::Error>> {
    _bundle(
        applications,
        "Music",
        "Nanika Music",
        &[
            ("eng", "Nanika NetEaseMusic"),
            ("en-posix", "Nanika POSIX Music"),
            ("jaw", "Nanika Alternate Music"),
            ("zh-Hans", "Nanika 网易云音乐"),
            ("ja", "Nanika ミュージック")
        ]
    )?;
    _bundle(
        applications,
        "Traditional",
        "",
        &[
            ("en_GB", "Nanika Traditional Music"),
            ("und", "Nanika Undetermined Music"),
            ("zh-US", "Nanika American Chinese"),
            ("zh-TW", "Nanika 網易雲音樂")
        ]
    )?;
    _bundle(
        applications,
        "Japanese",
        "Nanika 音楽箱",
        &[("ja", "Nanika 音楽箱")]
    )?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn _bundle(
    applications: &Path,
    id: &str,
    fallback: &str,
    names: &[(&str, &str)]
) -> Result<(), Box<dyn std::error::Error>> {
    let contents = applications.join(format!("Nanika {id}.app/Contents"));
    std::fs::create_dir_all(contents.join("MacOS"))?;
    let executable = contents.join("MacOS/Music");
    std::fs::write(&executable, "#!/bin/sh\nexit 0\n")?;
    std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o755))?;
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>app.nanika.e2e.{id}</string>
<key>CFBundleExecutable</key><string>Music</string>
<key>CFBundleName</key><string>{fallback}</string>
<key>CFBundleDevelopmentRegion</key><string>en</string>
</dict></plist>"#
        )
    )?;
    for (locale, name) in names {
        let resources = contents.join(format!("Resources/{locale}.lproj"));
        std::fs::create_dir_all(&resources)?;
        std::fs::write(
            resources.join("InfoPlist.strings"),
            format!("\"CFBundleDisplayName\" = \"{name}\";")
        )?;
    }
    Ok(())
}
