//! Nanika host entry point.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let reduced_motion = arguments
        .iter()
        .any(|argument| argument == "--reduced-motion");
    let background = arguments.iter().any(|argument| argument == "--background");
    let Some(paths) = nanika_storage::NanikaPaths::discover() else {
        nanika_platform::report_fatal_error("Nanika failed to resolve platform data directories");
        return;
    };
    let _diagnostics =
        match nanika_host::Diagnostics::initialize(&paths.app_data_root().join("logs")) {
            Ok(diagnostics) => Some(diagnostics),
            Err(error) => {
                nanika_platform::report_fatal_error(&format!(
                    "Nanika diagnostics are unavailable: {error}"
                ));
                None
            }
        };
    tracing::info!(background, "host starting");
    let identity = nanika_core::PROJECT_IDENTITY.bundle_id;
    let instance = match nanika_platform::acquire_instance(identity, paths.app_data_root()) {
        Ok(nanika_platform::InstanceRole::Primary(instance)) => instance,
        Ok(nanika_platform::InstanceRole::Secondary) => {
            if should_activate_existing(background)
                && let Err(error) =
                    nanika_platform::signal_activate(identity, paths.app_data_root())
            {
                report_fatal(&format!(
                    "Nanika failed to activate the existing host: {error}"
                ));
            }
            return;
        }
        Err(error) => {
            report_fatal(&format!(
                "Nanika failed to acquire the host instance: {error}"
            ));
            return;
        }
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Nanika")
            .with_inner_size([
                nanika_host::OVERLAY_WIDTH_POINTS,
                nanika_host::OVERLAY_HEIGHT_POINTS,
            ])
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
            Ok(Box::new(nanika_host::HostApp::with_instance_background(
                instance,
                reduced_motion,
                background,
            )))
        }),
    );
    if let Err(error) = result {
        report_fatal(&format!("Nanika failed to start: {error}"));
    }
    tracing::info!("host stopped");
}

fn report_fatal(message: &str) {
    tracing::error!(message, "fatal host error");
    nanika_platform::report_fatal_error(message);
}

fn should_activate_existing(background: bool) -> bool {
    !background
}

#[cfg(test)]
mod tests {
    use super::should_activate_existing;

    #[test]
    fn foreground_secondary_activates_the_existing_host() {
        assert!(should_activate_existing(false));
    }

    #[test]
    fn background_secondary_exits_without_activation() {
        assert!(!should_activate_existing(true));
    }
}
