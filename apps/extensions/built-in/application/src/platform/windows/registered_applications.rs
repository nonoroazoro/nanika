use crate::normalization::{normalize_name, stable_hash};
use crate::platform::DiscoveryInventory;
use crate::{ApplicationEntry, ApplicationEntryData, ApplicationError, ApplicationIconSource};
use windows::{Management::Deployment::PackageManager, core::HSTRING};

#[path = "RuntimeApartment.rs"]
mod runtime_apartment;
use runtime_apartment::RuntimeApartment;

const SETTING: &str = "application.builtin.windows.packaged";
const SOURCE: &str = "native:windows.packaged";

pub(in crate::platform) fn inventories(
    enabled: &std::collections::BTreeSet<String>,
    cancelled: &mut dyn FnMut() -> bool,
) -> Vec<DiscoveryInventory> {
    vec![DiscoveryInventory {
        key: SOURCE,
        // A disabled source is an authoritative empty inventory, including when
        // an unrelated filesystem source cannot resolve its root.
        entries: if enabled.contains(SETTING) {
            _read(cancelled)
        } else {
            Ok(Vec::new())
        },
    }]
}

fn _read(cancelled: &mut dyn FnMut() -> bool) -> Result<Vec<ApplicationEntry>, ApplicationError> {
    let mut read = || -> windows::core::Result<Vec<ApplicationEntry>> {
        let _apartment = RuntimeApartment::new()?;
        let manager = PackageManager::new()?;
        let packages = manager.FindPackagesByUserSecurityId(&HSTRING::new())?;
        let mut entries = Vec::new();
        for package in packages {
            if cancelled() {
                break;
            }
            if package.IsFramework()? || package.IsResourcePackage()? {
                continue;
            }
            let package_full_name = package.Id()?.FullName()?.to_string();
            // Enumerate every launchable application in the registered package;
            // the OS resolves localization and the package-family-qualified AUMID.
            for app in package.GetAppListEntriesAsync()?.join()? {
                if cancelled() {
                    break;
                }
                let id = app.AppUserModelId()?.to_string();
                let name = app.DisplayInfo()?.DisplayName()?.to_string();
                entries.push(ApplicationEntry::new(ApplicationEntryData {
                    entry_id: format!("app.{}", stable_hash(&["windows-packaged", &id])),
                    source_key: format!("{SOURCE}/{id}"),
                    normalized_name: normalize_name(&name),
                    normalized_tokens: normalize_name(&name),
                    display_name: name,
                    launch_kind: "windows-packaged".into(),
                    target_path: id.clone(),
                    arguments_json: r#"{"kind":"structured","values":[]}"#.into(),
                    icon_key: String::new(),
                    icon_source: Some(ApplicationIconSource::WindowsApplication {
                        app_user_model_id: id,
                        package_full_name: package_full_name.clone(),
                    }),
                    priority: 0,
                }));
            }
        }
        Ok(entries)
    };
    read().map_err(|error| {
        std::io::Error::other(format!("registered Windows applications: {error}")).into()
    })
}
