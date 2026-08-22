//! Nanika host entry point.

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let reduced_motion = arguments
        .iter()
        .any(|argument| argument == "--reduced-motion");
    let background = arguments.iter().any(|argument| argument == "--background");
    let Some(paths) = nanika_storage::NanikaPaths::discover() else {
        eprintln!("Nanika failed to resolve platform data directories");
        return;
    };
    let identity = nanika_core::PROJECT_IDENTITY.bundle_id;
    let instance = match nanika_platform::acquire_instance(identity, paths.app_data_root()) {
        Ok(nanika_platform::InstanceRole::Primary(instance)) => instance,
        Ok(nanika_platform::InstanceRole::Secondary) => {
            if should_activate_existing(background)
                && let Err(error) =
                    nanika_platform::signal_activate(identity, paths.app_data_root())
            {
                eprintln!("Nanika failed to activate the existing host: {error}");
            }
            return;
        }
        Err(error) => {
            eprintln!("Nanika failed to acquire the host instance: {error}");
            return;
        }
    };
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
            Ok(Box::new(nanika_host::HostApp::with_instance_background(
                instance,
                reduced_motion,
                background,
            )))
        }),
    );
    if let Err(error) = result {
        eprintln!("Nanika failed to start: {error}");
    }
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
