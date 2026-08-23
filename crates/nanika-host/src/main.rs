//! Nanika host entry point.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    let arguments = std::env::args().collect::<Vec<_>>();
    let reduced_motion = arguments
        .iter()
        .any(|argument| argument == "--reduced-motion");
    let background = arguments.iter().any(|argument| argument == "--background");
    let Some(paths) = nanika_storage::NanikaPaths::discover() else {
        report_fatal(nanika_host::HostDiagnostic::new(
            nanika_host::DiagnosticCode::PlatformUnavailable,
            "resolve platform data directories",
            "Nanika could not resolve its data directories. Check the current user profile.",
        ));
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
                report_fatal(nanika_host::HostDiagnostic::from_error(
                    nanika_host::DiagnosticCode::PlatformUnavailable,
                    "activate existing host",
                    "Nanika could not activate the existing window. Quit the running instance and try again.",
                    error,
                ));
            }
            return;
        }
        Err(error) => {
            report_fatal(nanika_host::HostDiagnostic::from_error(
                nanika_host::DiagnosticCode::PlatformUnavailable,
                "acquire host instance",
                "Nanika could not acquire instance ownership. Restart Nanika and try again.",
                error,
            ));
            return;
        }
    };
    let _diagnostics =
        match nanika_host::Diagnostics::initialize(&paths.app_data_root().join("logs")) {
            Ok(diagnostics) => Some(diagnostics),
            Err(error) => {
                let diagnostic = nanika_host::HostDiagnostic::from_message(
                    nanika_host::DiagnosticCode::DiagnosticsUnavailable,
                    "initialize diagnostic logging",
                    "Nanika started without diagnostic logging. Check app-data permissions.",
                    error,
                );
                diagnostic.record_warning();
                nanika_platform::report_fatal_error(diagnostic.user_message());
                None
            }
        };
    tracing::info!(background, "host starting");
    let result = nanika_host::HostRunner::run(instance, reduced_motion);
    if let Err(error) = result {
        report_fatal(nanika_host::HostDiagnostic::from_message(
            nanika_host::DiagnosticCode::PlatformUnavailable,
            "create host window",
            "Nanika could not create its window. Check graphics drivers and diagnostics.",
            error.to_string(),
        ));
    }
    tracing::info!("host stopped");
}

fn report_fatal(diagnostic: nanika_host::HostDiagnostic) {
    diagnostic.record_error();
    nanika_platform::report_fatal_error(diagnostic.user_message());
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
