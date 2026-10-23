use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::Manager;

#[path = "VerificationRun.rs"]
mod verification_run;
use verification_run::VerificationRun;
#[path = "ProcessSample.rs"]
mod process_sample;
#[path = "ResourceMonitor.rs"]
mod resource_monitor;
#[path = "ResourceSnapshot.rs"]
mod resource_snapshot;

#[tauri::command]
async fn verification_resources(
    window: tauri::WebviewWindow,
    monitor: tauri::State<'_, Arc<Mutex<resource_monitor::ResourceMonitor>>>,
    idle_ms: u64,
    hidden: bool
) -> Result<[resource_snapshot::ResourceSnapshot; 2], String> {
    if idle_ms > 5000 {
        return Err("Idle sample exceeds five seconds".to_owned());
    }
    if hidden {
        window.hide().map_err(|error| error.to_string())?;
        window.as_ref().hide().map_err(|error| error.to_string())?;
    }
    let monitor = Arc::clone(monitor.inner());
    let result = tauri::async_runtime::spawn_blocking(move || {
        if hidden {
            // Let native visibility delivery and worker teardown settle before sampling idle.
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        let mut monitor = monitor.lock().map_err(|error| error.to_string())?;
        let before = monitor.snapshot();
        std::thread::sleep(std::time::Duration::from_millis(idle_ms));
        Ok([before, monitor.snapshot()])
    })
    .await
    .map_err(|error| error.to_string());
    if hidden {
        window.as_ref().show().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
    }
    result?
}

#[tauri::command]
fn verification_report(
    window: tauri::WebviewWindow,
    run: tauri::State<'_, VerificationRun>,
    mut report: serde_json::Value
) -> Result<(), String> {
    let object = report.as_object_mut().ok_or("Expected a report object")?;
    object.insert("nativeEnvironment".to_owned(), serde_json::json!({
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "osVersion": sysinfo::System::os_version(),
        "webviewVersion": tauri::webview_version().ok(),
        "monitor": window.current_monitor().ok().flatten(),
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "memoryScope": "Per-process resident bytes, not summed physical or total App memory. Only the host and OS-reported descendants are attributed. macOS XPC WebKit processes may not be descendants.",
        "cpuScope": "Cumulative CPU milliseconds per listed process; host samples include resource collection overhead."
    }));
    let bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 || run.completed.swap(true, Ordering::SeqCst) {
        return Err("Invalid or repeated verification report".to_owned());
    }
    let result = std::fs::write(&run.output, bytes).map_err(|error| error.to_string());
    window
        .app_handle()
        .exit(if result.is_ok() && report.get("error").is_none() {
            0
        } else {
            1
        });
    result
}

#[tauri::command]
fn verification_progress(message: String) -> Result<(), String> {
    if message.len() > 512 {
        return Err("Progress message is too long".to_owned());
    }
    println!("{message}");
    Ok(())
}

fn main() {
    if let Err(error) = _run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn _run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [scenario, directory] = args.as_slice() else {
        return Err(
            "Usage: nanika-verify <syntax:rendering|syntax:latency|syntax:lifecycle|localization:switching> <new-output-directory>".into()
        );
    };
    let scenario = scenario
        .to_str()
        .filter(|value| {
            matches!(
                *value,
                "syntax:rendering"
                    | "syntax:latency"
                    | "syntax:lifecycle"
                    | "localization:switching"
            )
        })
        .ok_or("Unknown native scenario")?;
    let directory = PathBuf::from(directory);
    // A new directory ensures the run cannot reuse or overwrite user state.
    std::fs::create_dir(&directory)?;
    let directory = directory.canonicalize()?;
    let surface = if scenario == "localization:switching" {
        "launcher"
    } else {
        "syntax"
    };
    let url = format!("index.html?syntax=on&surface={surface}&native=1&scenario={scenario}");
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(
            resource_monitor::ResourceMonitor::new()
        )))
        .manage(VerificationRun {
            output: directory.join("report.json"),
            completed: AtomicBool::new(false)
        })
        .invoke_handler(tauri::generate_handler![
            verification_report,
            verification_progress,
            verification_resources
        ])
        .setup(move |app| {
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "verification",
                tauri::WebviewUrl::App(url.into())
            )
            .title("Nanika Verification")
            .inner_size(760.0, 520.0)
            .decorations(false)
            .always_on_top(true)
            .incognito(true)
            .data_directory(directory.join("webview"))
            .build()?;
            window.set_focus()?;
            let app = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(120));
                eprintln!("Native verification exceeded 120 seconds");
                app.exit(1);
            });
            Ok(())
        })
        .run(tauri::tauri_build_context!())?;
    Ok(())
}
