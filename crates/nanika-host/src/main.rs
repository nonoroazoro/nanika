//! Nanika host entry point.

fn main() {
    let Some(paths) = nanika_storage::NanikaPaths::discover() else {
        eprintln!("Nanika failed to resolve platform data directories");
        return;
    };
    let identity = nanika_core::PROJECT_IDENTITY.bundle_id;
    let instance = match nanika_platform::acquire_instance(identity, paths.app_data_root()) {
        Ok(nanika_platform::InstanceRole::Primary(instance)) => instance,
        Ok(nanika_platform::InstanceRole::Secondary) => {
            if let Err(error) = nanika_platform::signal_activate(identity, paths.app_data_root()) {
                eprintln!("Nanika failed to activate the existing host: {error}");
            }
            return;
        }
        Err(error) => {
            eprintln!("Nanika failed to acquire the host instance: {error}");
            return;
        }
    };
    let reduced_motion = std::env::args().any(|argument| argument == "--reduced-motion");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Nanika")
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([480.0, 240.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_visible(false),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Nanika",
        options,
        Box::new(move |_creation_context| {
            Ok(Box::new(nanika_host::HostApp::with_instance(
                instance,
                reduced_motion,
            )))
        }),
    );
    if let Err(error) = result {
        eprintln!("Nanika failed to start: {error}");
    }
}
