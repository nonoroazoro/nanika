use std::collections::BTreeMap;

use eframe::egui;
use nanika_platform::StartupStatus;
use nanika_protocol::{
    SettingColumn, SettingColumnControl, SettingControl, SettingField, SettingUpdate, SettingValue,
};

use crate::{SettingsAction, SettingsState};

const RECORDS_PER_PAGE: usize = 20;

pub(crate) fn show_settings(
    context: &egui::Context,
    state: &mut SettingsState,
) -> Vec<SettingsAction> {
    if !state.visible {
        return Vec::new();
    }
    let mut actions = Vec::new();
    context.show_viewport_immediate(
        egui::ViewportId::from_hash_of("nanika.settings"),
        egui::ViewportBuilder::default()
            .with_title("Nanika Settings")
            .with_inner_size([760.0, 680.0])
            .with_min_inner_size([560.0, 420.0]),
        |child, _class| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(24)))
                .show(child, |ui| {
                    if ui.input(|input| input.viewport().close_requested()) {
                        actions.push(SettingsAction::Close);
                        return;
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Nanika Settings");
                        ui.add_space(16.0);
                        ui.strong("Host");
                        ui.add_enabled_ui(state.runtime_ready && !state.saving_host, |ui| {
                            ui.label("Global hotkey");
                            ui.text_edit_singleline(&mut state.hotkey);
                            ui.checkbox(&mut state.reduced_motion, "Reduced motion");
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    state.runtime_ready && !state.saving_host,
                                    egui::Button::new("Save host settings"),
                                )
                                .clicked()
                            {
                                actions.push(SettingsAction::SaveHost);
                            }
                            if state.saving_host {
                                ui.spinner();
                            }
                        });

                        ui.add_space(16.0);
                        ui.strong("Start at login");
                        ui.horizontal(|ui| {
                            let loading = !state.runtime_ready || state.startup_response.is_some();
                            ui.label(startup_label(state.startup_status, loading));
                            if loading {
                                ui.spinner();
                            } else if let Some((label, enabled)) =
                                startup_action(state.startup_status)
                                && ui.button(label).clicked()
                            {
                                actions.push(SettingsAction::SetStartup(enabled));
                            }
                        });

                        for (extension_id, contribution) in &mut state.drafts {
                            if contribution.fields.is_empty() {
                                continue;
                            }
                            ui.add_space(24.0);
                            ui.separator();
                            ui.add_space(12.0);
                            ui.heading(&contribution.title);
                            let saving = state.pending_extensions.contains_key(extension_id);
                            let mut changed = false;
                            ui.add_enabled_ui(!saving, |ui| {
                                for field in &mut contribution.fields {
                                    ui.add_space(12.0);
                                    changed |= render_field(ui, extension_id, field);
                                }
                            });
                            if changed {
                                state.dirty.insert(extension_id.clone());
                            }
                            if ui
                                .add_enabled(
                                    state.dirty.contains(extension_id) && !saving,
                                    egui::Button::new("Apply"),
                                )
                                .clicked()
                            {
                                actions.push(SettingsAction::SaveExtension {
                                    extension_id: extension_id.clone(),
                                    updates: contribution
                                        .fields
                                        .iter()
                                        .map(|field| SettingUpdate {
                                            key: field.key.clone(),
                                            value: field.value.clone(),
                                        })
                                        .collect(),
                                });
                            }
                            if saving {
                                ui.spinner();
                            }
                        }

                        if let Some(error) = &state.error {
                            ui.add_space(16.0);
                            ui.colored_label(
                                egui::Color32::from_rgb(242, 145, 145),
                                error.user_message(),
                            );
                        }
                    });
                });
        },
    );
    actions
}

fn render_field(ui: &mut egui::Ui, extension_id: &str, field: &mut SettingField) -> bool {
    ui.strong(&field.title);
    if let Some(description) = &field.description {
        ui.label(description);
    }
    match (&field.control, &mut field.value) {
        (SettingControl::Toggle, SettingValue::Boolean { value }) => {
            ui.checkbox(value, "Enabled").changed()
        }
        (SettingControl::Text { placeholder, .. }, SettingValue::Text { value }) => ui
            .add(egui::TextEdit::singleline(value).hint_text(placeholder.as_deref().unwrap_or("")))
            .changed(),
        (
            SettingControl::StringList {
                placeholder,
                max_items,
                ..
            },
            SettingValue::StringList { values },
        ) => render_string_list(ui, values, placeholder.as_deref(), *max_items as usize),
        (SettingControl::RecordTable { columns, max_rows }, SettingValue::Records { rows }) => {
            render_records(
                ui,
                extension_id,
                &field.key,
                columns,
                *max_rows as usize,
                rows,
            )
        }
        _ => {
            ui.colored_label(egui::Color32::RED, "Invalid settings schema");
            false
        }
    }
}

fn render_string_list(
    ui: &mut egui::Ui,
    values: &mut Vec<String>,
    placeholder: Option<&str>,
    maximum: usize,
) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (index, value) in values.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::TextEdit::singleline(value)
                        .hint_text(placeholder.unwrap_or("Value"))
                        .desired_width(f32::INFINITY),
                )
                .changed();
            if ui.small_button("Remove").clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        values.remove(index);
        changed = true;
    }
    if values.len() < maximum && ui.small_button("Add").clicked() {
        values.push(String::new());
        changed = true;
    }
    changed
}

fn render_records(
    ui: &mut egui::Ui,
    extension_id: &str,
    field_key: &str,
    columns: &[SettingColumn],
    maximum: usize,
    rows: &mut Vec<BTreeMap<String, SettingValue>>,
) -> bool {
    let page_id = egui::Id::new(("settings-page", extension_id, field_key));
    let page_count = rows.len().max(1).div_ceil(RECORDS_PER_PAGE);
    let mut page = ui
        .ctx()
        .data_mut(|data| data.get_temp::<usize>(page_id).unwrap_or(0))
        .min(page_count.saturating_sub(1));
    ui.horizontal(|ui| {
        if ui
            .add_enabled(page > 0, egui::Button::new("Previous"))
            .clicked()
        {
            page = page.saturating_sub(1);
        }
        ui.label(format!("Page {} of {}", page + 1, page_count));
        if ui
            .add_enabled(page + 1 < page_count, egui::Button::new("Next"))
            .clicked()
        {
            page += 1;
        }
    });
    ui.ctx().data_mut(|data| data.insert_temp(page_id, page));

    let start = page * RECORDS_PER_PAGE;
    let end = (start + RECORDS_PER_PAGE).min(rows.len());
    let mut changed = false;
    let mut remove = None;
    for index in start..end {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.strong(format!("Item {}", index + 1));
                if ui.small_button("Remove").clicked() {
                    remove = Some(index);
                }
            });
            if let Some(row) = rows.get_mut(index) {
                for column in columns {
                    ui.label(&column.title);
                    if let Some(value) = row.get_mut(&column.key) {
                        changed |= render_column(ui, &column.control, value);
                    }
                }
            }
        });
    }
    if let Some(index) = remove {
        rows.remove(index);
        changed = true;
    }
    if rows.len() < maximum && ui.small_button("Add item").clicked() {
        rows.push(empty_record(columns));
        changed = true;
    }
    changed
}

fn render_column(
    ui: &mut egui::Ui,
    control: &SettingColumnControl,
    value: &mut SettingValue,
) -> bool {
    match (control, value) {
        (SettingColumnControl::Text { placeholder, .. }, SettingValue::Text { value }) => ui
            .add(egui::TextEdit::singleline(value).hint_text(placeholder.as_deref().unwrap_or("")))
            .changed(),
        (
            SettingColumnControl::StringList {
                placeholder,
                max_items,
            },
            SettingValue::StringList { values },
        ) => render_string_list(ui, values, placeholder.as_deref(), *max_items as usize),
        _ => false,
    }
}

fn empty_record(columns: &[SettingColumn]) -> BTreeMap<String, SettingValue> {
    columns
        .iter()
        .map(|column| {
            let value = match column.control {
                SettingColumnControl::Text { .. } => SettingValue::Text {
                    value: String::new(),
                },
                SettingColumnControl::StringList { .. } => {
                    SettingValue::StringList { values: Vec::new() }
                }
            };
            (column.key.clone(), value)
        })
        .collect()
}

fn startup_label(status: Option<StartupStatus>, loading: bool) -> &'static str {
    match status {
        Some(StartupStatus::Disabled) => "Disabled",
        Some(StartupStatus::Enabled) => "Enabled",
        Some(StartupStatus::RequiresApproval) => "Requires approval in System Settings",
        Some(StartupStatus::NeedsRepair) => "Existing registration needs repair",
        Some(StartupStatus::NotFound) => "Application bundle not found",
        None if loading => "Checking",
        None => "Unavailable",
    }
}

pub(crate) fn startup_action(status: Option<StartupStatus>) -> Option<(&'static str, bool)> {
    match status {
        Some(StartupStatus::Disabled | StartupStatus::NeedsRepair) => Some(("Enable", true)),
        Some(StartupStatus::Enabled) => Some(("Disable", false)),
        Some(StartupStatus::RequiresApproval) => Some(("Open System Settings", true)),
        Some(StartupStatus::NotFound) | None => None,
    }
}
