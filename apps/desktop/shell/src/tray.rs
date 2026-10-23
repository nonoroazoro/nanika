use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

pub(crate) fn install(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let localization = app
        .state::<crate::host_settings::HostSettings>()
        .snapshot()
        .localization;
    let resources = crate::host_resources::resources()?;
    let label = |id| {
        resources.format(
            localization.locale,
            &nanika_protocol::message(id, []),
            nanika_platform::target_platform().as_str(),
            128,
            false
        )
    };
    let open = MenuItem::with_id(app, "open", label("tray-open")?, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", label("tray-quit")?, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &separator, &quit])?;

    TrayIconBuilder::with_id("nanika")
        .tooltip("Nanika")
        .icon(tray_icon())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Err(error) = crate::show_launcher(app) {
                    tracing::error!(%error, "tray command could not show launcher");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && let Err(error) = crate::toggle_launcher(tray.app_handle())
            {
                tracing::error!(%error, "tray click could not toggle launcher");
            }
        })
        .build(app)?;
    app.manage(crate::NativeLocalization::new(open, quit));
    Ok(())
}

fn tray_icon() -> tauri::image::Image<'static> {
    const SIDE: usize = 32;
    let mut rgba = vec![0_u8; SIDE * SIDE * 4];
    for y in 7_usize..25 {
        for x in 7_usize..25 {
            let diagonal = x.abs_diff(y) <= 2 || x.abs_diff(SIDE - 1 - y) <= 2;
            let vertical = matches!(x, 7..=10 | 21..=24);
            if diagonal || vertical {
                let offset = (y * SIDE + x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&[30, 30, 34, 255]);
            }
        }
    }
    tauri::image::Image::new_owned(rgba, SIDE as u32, SIDE as u32)
}
