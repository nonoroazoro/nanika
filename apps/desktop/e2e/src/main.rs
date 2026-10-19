use std::io::{Read, Write};
use std::path::PathBuf;

mod clipboard;
#[allow(unsafe_code)]
mod ime;
#[allow(unsafe_code)]
mod input;
mod lifecycle;
#[allow(unsafe_code)]
mod native_dialog;

fn main() -> std::process::ExitCode {
    match _run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn _run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        #[cfg(target_os = "windows")]
        [command, pid] if command == "picker" || command == "cancel-picker" => {
            println!("{}", native_dialog::query_process(pid.parse()?, command == "cancel-picker")?);
            Ok(())
        },
        [command, pid] if command == "ime" => ime::run(pid.parse()?).map_err(Into::into),
        [command, value] if command == "clipboard" => clipboard::run(value).map_err(Into::into),
        [command, key] if command == "key" => {
            let keys = input::press(key)?;
            println!("posted");
            std::io::stdout().flush()?;
            let mut byte = [0];
            if std::io::stdin().read(&mut byte)? != 0 {
                return Err("Expected EOF after native input acknowledgement".into());
            }
            // The UI can react to key-down before the OS has delivered the chord's releases.
            input::wait_for_release(&keys)?;
            Ok(())
        },
        [command, directory, identity, pid] if command == "activate" => {
            nanika_platform::signal_activate(identity, &PathBuf::from(directory).join("data"), Some(pid.parse()?))?;
            Ok(())
        }
        [command, directory, identity, port] if command == "run" => {
            _launch(PathBuf::from(directory), identity, port.parse()?)
        }
        _ => Err("Usage: nanika-e2e <run|activate> <directory> <identity> <port|pid>, nanika-e2e key <launcher|settings|switch-app|escape|enter|down|up|n|i>, nanika-e2e clipboard <marker>, or nanika-e2e ime <pid>".into())
    }
}

fn _launch(
    directory: PathBuf,
    identity: &str,
    port: u16
) -> Result<(), Box<dyn std::error::Error>> {
    if !identity.starts_with("app.nanika.e2e.") || port == 0 {
        return Err("Expected a test instance identity and a nonzero driver port".into());
    }
    // Refuse existing state instead of risking another session's data.
    std::fs::create_dir(&directory)?;
    let directory = directory.canonicalize()?;
    let paths = nanika_storage::NanikaPaths::from_roots(
        directory.join("data"),
        directory.join("cache"),
        directory.join("config")
    );
    let store = nanika_config::ConfigStore::open(paths.app_data_root(), paths.config_root())?;
    let mut preferences = nanika_config::LauncherPreferences::load(&store, "Ctrl+Alt+Shift+F12")?;
    preferences.launcher_shortcut = "Ctrl+Alt+Shift+F12".into();
    preferences.save(&store)?;
    let mut context = nanika_desktop::context();
    context.config_mut().identifier = identity.to_owned();
    for window in &mut context.config_mut().app.windows {
        window.incognito = true;
        window.data_directory = Some(directory.join("webview").join(&window.label));
    }
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_wdio_webdriver::init_with_port(port))
        .plugin(lifecycle::plugin(directory));
    nanika_desktop::run_with(paths, builder, context).map_err(Into::into)
}
