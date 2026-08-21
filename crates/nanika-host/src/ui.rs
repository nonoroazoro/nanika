use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use nanika_platform::{HotkeyRegistration, PlatformError};

const FRAME_INTERVAL: Duration = Duration::from_millis(8);

fn default_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL), Code::Space)
}

pub struct HostApp {
    hotkey: Option<HotkeyRegistration>,
    hotkey_error: Option<String>,
    events: mpsc::Receiver<GlobalHotKeyEvent>,
    context_slot: Arc<Mutex<Option<egui::Context>>>,
    query: String,
    history: Vec<String>,
    history_cursor: Option<usize>,
    overlay_visible: bool,
    motion: OverlayMotion,
    visuals_configured: bool,
}

impl HostApp {
    pub fn new() -> Self {
        Self::new_with_reduced_motion(false)
    }

    pub fn new_with_reduced_motion(reduced_motion: bool) -> Self {
        let (sender, events) = mpsc::channel();
        let context_slot: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));
        let event_context = Arc::clone(&context_slot);
        GlobalHotKeyEvent::set_event_handler(Some(move |event| {
            let _ = sender.send(event);
            if let Some(context) = event_context
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_ref()
            {
                context.request_repaint();
            }
        }));

        let (hotkey, hotkey_error) = match HotkeyRegistration::register(default_hotkey()) {
            Ok(hotkey) => (Some(hotkey), None),
            Err(error) => (None, Some(platform_error_message(error))),
        };

        Self {
            hotkey,
            hotkey_error,
            events,
            context_slot,
            query: String::new(),
            history: Vec::new(),
            history_cursor: None,
            overlay_visible: false,
            motion: OverlayMotion::new(reduced_motion),
            visuals_configured: false,
        }
    }

    fn handle_events(&mut self, context: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            if event.state == HotKeyState::Pressed
                && self
                    .hotkey
                    .as_ref()
                    .is_some_and(|hotkey| hotkey.id() == event.id)
            {
                self.open_overlay(context);
            }
        }
    }

    fn open_overlay(&mut self, context: &egui::Context) {
        self.overlay_visible = true;
        self.motion.set_target(true);
        context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        context.send_viewport_cmd(egui::ViewportCommand::Focus);
        context.request_repaint();
    }

    fn close_overlay(&mut self, context: &egui::Context) {
        self.motion.set_target(false);
        context.request_repaint();
    }

    fn navigate_history(&mut self, direction: HistoryDirection) {
        if self.history.is_empty() {
            return;
        }
        match direction {
            HistoryDirection::Older => {
                let next = self
                    .history_cursor
                    .map_or(self.history.len() - 1, |index| index.saturating_sub(1));
                self.history_cursor = Some(next);
                self.query = self.history[next].clone();
            }
            HistoryDirection::Newer => match self.history_cursor {
                Some(index) if index + 1 < self.history.len() => {
                    let next = index + 1;
                    self.history_cursor = Some(next);
                    self.query = self.history[next].clone();
                }
                Some(_) => {
                    self.history_cursor = None;
                    self.query.clear();
                }
                None => {}
            },
        }
    }

    fn submit_query(&mut self) {
        let query = self.query.trim();
        if query.is_empty() {
            return;
        }
        self.history.retain(|item| item != query);
        self.history.push(query.to_owned());
        self.history_cursor = None;
    }
}

impl Default for HostApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for HostApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        if !self.visuals_configured {
            let mut style = (*context.style_of(egui::Theme::Dark)).clone();
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 8.0);
            context.set_style_of(egui::Theme::Dark, style);
            self.visuals_configured = true;
        }
        *self
            .context_slot
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(context.clone());
        self.handle_events(&context);

        if !self.overlay_visible && !self.motion.is_active() {
            context.request_repaint_after(Duration::from_secs(3600));
            return;
        }

        let moving = self.motion.advance();
        if moving {
            context.request_repaint_after(FRAME_INTERVAL);
        } else if !self.motion.target_visible() && self.overlay_visible {
            self.overlay_visible = false;
            context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if !self.overlay_visible {
            return;
        }

        let input_id = egui::Id::new("nanika.query");
        let alpha = (self.motion.value() * 255.0).round() as u8;
        let panel_fill = egui::Color32::from_rgba_unmultiplied(20, 22, 30, alpha);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(panel_fill)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new("NANIKA")
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::from_rgb(168, 176, 198)),
                    );
                    ui.add_space(12.0);
                    let response = ui.add_sized(
                        [ui.available_width(), 54.0],
                        egui::TextEdit::singleline(&mut self.query)
                            .id(input_id)
                            .hint_text("Type a command, app, calculation, or keyword")
                            .font(egui::TextStyle::Heading),
                    );
                    if self.motion.target_visible() {
                        context.memory_mut(|memory| memory.request_focus(input_id));
                    }

                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        self.submit_query();
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                        self.navigate_history(HistoryDirection::Older);
                    } else if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                        self.navigate_history(HistoryDirection::Newer);
                    } else if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        self.close_overlay(&context);
                    }

                    ui.add_space(16.0);
                    if let Some(error) = &self.hotkey_error {
                        ui.colored_label(egui::Color32::from_rgb(242, 145, 145), error);
                    } else if self.query.is_empty() {
                        ui.label(
                            egui::RichText::new("No extensions enabled")
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Search extensions to see results")
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                    }
                });
            });
    }
}

#[derive(Debug, Default)]
struct OverlayMotion {
    value: f32,
    target: f32,
    last_frame: Option<Instant>,
    reduced_motion: bool,
}

impl OverlayMotion {
    fn new(reduced_motion: bool) -> Self {
        Self {
            reduced_motion,
            ..Self::default()
        }
    }

    fn set_target(&mut self, visible: bool) {
        self.target = if visible { 1.0 } else { 0.0 };
        self.last_frame = Some(Instant::now());
    }

    fn advance(&mut self) -> bool {
        if self.reduced_motion {
            self.value = self.target;
            self.last_frame = None;
            return false;
        }
        let now = Instant::now();
        let elapsed = self
            .last_frame
            .replace(now)
            .map_or(FRAME_INTERVAL, |last| now.saturating_duration_since(last));
        let distance = self.target - self.value;
        if distance.abs() < 0.001 {
            self.value = self.target;
            return false;
        }
        let amount = (elapsed.as_secs_f32() * 12.0).min(1.0);
        let eased = amount * amount * (3.0 - 2.0 * amount);
        self.value += distance * eased;
        true
    }

    fn value(&self) -> f32 {
        self.value
    }

    fn target_visible(&self) -> bool {
        self.target > 0.5
    }

    fn is_active(&self) -> bool {
        (self.value - self.target).abs() >= 0.001
    }
}

enum HistoryDirection {
    Older,
    Newer,
}

fn platform_error_message(error: PlatformError) -> String {
    format!("Global hotkey unavailable: {error}")
}

#[cfg(test)]
mod tests {
    use super::OverlayMotion;

    #[test]
    fn reduced_motion_jumps_to_the_target_without_repaint_loop() {
        let mut motion = OverlayMotion::new(true);
        motion.set_target(true);
        assert!(!motion.advance());
        assert_eq!(motion.value(), 1.0);
        assert!(!motion.is_active());
    }
}
