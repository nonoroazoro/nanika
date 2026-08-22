use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use eframe::egui;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use nanika_config::ConfigStore;
use nanika_platform::{HotkeyRegistration, PlatformError, SingleInstance};
use nanika_search::{
    InputHistory, MAX_QUERY_CHARS, SearchHandle, SearchOwner, SearchSnapshot, UsageKey, UsageMap,
    UsageStat, normalize_history_key,
};
use nanika_storage::{ExtensionKind, NanikaPaths, SearchStorageWorker};

use crate::{
    ExtensionProcess, ExtensionSearchCoordinator, HistoryDirection, HostEvent, HostRuntime,
    OverlayMotion,
};

const FRAME_INTERVAL: Duration = Duration::from_micros(8_333);
const MAX_VISIBLE_RESULTS: usize = 8;

fn default_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL), Code::Space)
}

pub struct HostApp {
    hotkey: Option<HotkeyRegistration>,
    hotkey_error: Option<String>,
    events: mpsc::Receiver<HostEvent>,
    context_slot: Arc<Mutex<Option<egui::Context>>>,
    instance: Option<SingleInstance>,
    instance_bridge: Option<JoinHandle<()>>,
    runtime_receiver: Option<mpsc::Receiver<HostRuntime>>,
    runtime_thread: Option<JoinHandle<()>>,
    query: String,
    history: InputHistory,
    config: Option<ConfigStore>,
    search_owner: Option<SearchOwner>,
    search: Option<SearchHandle>,
    search_notifier_configured: bool,
    search_generation: u64,
    search_snapshot: Option<Arc<SearchSnapshot>>,
    selected_index: usize,
    extension_search: ExtensionSearchCoordinator,
    storage: Option<SearchStorageWorker>,
    runtime_error: Option<String>,
    search_error: Option<String>,
    operation_error: Option<String>,
    action_error: Option<String>,
    overlay_visible: bool,
    focus_pending: bool,
    focus_observed: bool,
    motion: OverlayMotion,
    visuals_configured: bool,
    reveal_on_first_frame: bool,
}

impl HostApp {
    pub fn new() -> Self {
        Self::build(None, false)
    }

    pub fn new_with_reduced_motion(reduced_motion: bool) -> Self {
        Self::build(None, reduced_motion)
    }

    pub fn with_instance(instance: SingleInstance, reduced_motion: bool) -> Self {
        Self::build(Some(instance), reduced_motion)
    }

    fn build(mut instance: Option<SingleInstance>, reduced_motion: bool) -> Self {
        let (sender, events) = mpsc::sync_channel(16);
        let context_slot: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));
        let event_context = Arc::clone(&context_slot);
        let hotkey_events = sender.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event| {
            let _ = hotkey_events.try_send(HostEvent::Hotkey(event));
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

        let (instance_bridge, mut startup_error) = if let Some(instance) = instance.as_mut() {
            match instance.take_activations() {
                Ok(activations) => {
                    let activation_events = sender;
                    let activation_context = Arc::clone(&context_slot);
                    match std::thread::Builder::new()
                        .name("nanika-instance-bridge".to_owned())
                        .spawn(move || {
                            while activations.recv().is_ok() {
                                let _ = activation_events.try_send(HostEvent::Activate);
                                if let Some(context) = activation_context
                                    .lock()
                                    .unwrap_or_else(|error| error.into_inner())
                                    .as_ref()
                                {
                                    context.request_repaint();
                                }
                            }
                        }) {
                        Ok(thread) => (Some(thread), None),
                        Err(error) => (None, Some(error.to_string())),
                    }
                }
                Err(error) => (None, Some(error.to_string())),
            }
        } else {
            (None, None)
        };

        let (runtime_sender, runtime_receiver) = mpsc::sync_channel(1);
        let runtime_context = Arc::clone(&context_slot);
        let runtime_thread = match std::thread::Builder::new()
            .name("nanika-runtime-initializer".to_owned())
            .spawn(move || {
                let _ = runtime_sender.send(initialize_search_runtime());
                if let Some(context) = runtime_context
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                {
                    context.request_repaint();
                }
            }) {
            Ok(thread) => Some(thread),
            Err(error) => {
                startup_error = Some(error.to_string());
                None
            }
        };

        let reveal_on_first_frame = hotkey.is_none();
        Self {
            hotkey,
            hotkey_error,
            events,
            context_slot,
            instance,
            instance_bridge,
            runtime_receiver: Some(runtime_receiver),
            runtime_thread,
            query: String::new(),
            history: InputHistory::new(50),
            config: None,
            search_owner: None,
            search: None,
            search_notifier_configured: false,
            search_generation: 0,
            search_snapshot: None,
            selected_index: 0,
            extension_search: ExtensionSearchCoordinator::default(),
            storage: None,
            runtime_error: startup_error,
            search_error: None,
            operation_error: None,
            action_error: None,
            overlay_visible: false,
            focus_pending: false,
            focus_observed: false,
            motion: OverlayMotion::new(reduced_motion),
            visuals_configured: false,
            reveal_on_first_frame,
        }
    }

    fn handle_events(&mut self, context: &egui::Context) {
        let mut activate = false;
        while let Ok(event) = self.events.try_recv() {
            match event {
                HostEvent::Activate => activate = true,
                HostEvent::Hotkey(event)
                    if event.state == HotKeyState::Pressed
                        && self
                            .hotkey
                            .as_ref()
                            .is_some_and(|hotkey| hotkey.id() == event.id) =>
                {
                    activate = true;
                }
                HostEvent::Hotkey(_) => {}
            }
        }
        if activate {
            self.open_overlay(context);
        }
    }

    fn open_overlay(&mut self, context: &egui::Context) {
        self.overlay_visible = true;
        self.focus_pending = true;
        self.focus_observed = false;
        self.motion.set_target(true);
        context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        context.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.begin_search();
        context.request_repaint();
    }

    fn close_overlay(&mut self, context: &egui::Context) {
        self.motion.set_target(false);
        context.request_repaint();
    }

    fn navigate_history(&mut self, direction: HistoryDirection) {
        let query = match direction {
            HistoryDirection::Older => self.history.older(&self.query),
            HistoryDirection::Newer => self.history.newer(),
        };
        if let Some(query) = query {
            self.query = query;
            self.begin_search();
        }
    }

    fn submit_query(&mut self, context: &egui::Context) {
        let query = self.query.trim().to_owned();
        if query.is_empty() {
            return;
        }
        self.action_error = None;
        let selected = self
            .search_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.generation == self.search_generation)
            .and_then(|snapshot| snapshot.results.get(self.selected_index))
            .map(|result| result.candidate.clone());
        let Some(candidate) = selected else {
            self.operation_error = Some("no action is ready".to_owned());
            return;
        };
        let extension_id = candidate.extension_id().to_owned();
        self.history.record(&query);
        if let Some(storage) = &self.storage
            && let Err(error) = storage.record_history(
                normalize_history_key(&query),
                query.clone(),
                unix_timestamp_millis(),
            )
        {
            self.operation_error = Some(error.to_string());
        }
        match self.extension_search.invoke(
            &extension_id,
            self.search_generation,
            candidate.entry_id(),
            candidate.action_id(),
            query,
        ) {
            Ok(()) => self.close_overlay(context),
            Err(error) => self.action_error = Some(error.to_string()),
        }
    }

    fn begin_search(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        match search.begin_query(self.query.clone()) {
            Ok(generation) => {
                self.search_generation = generation;
                self.selected_index = 0;
                self.operation_error = None;
                self.extension_search.query(generation, &self.query);
            }
            Err(error) => {
                self.search_generation = 0;
                self.search_snapshot = None;
                self.selected_index = 0;
                self.operation_error = Some(error.to_string());
            }
        }
    }

    fn refresh_search_snapshot(&mut self) {
        if let Some(snapshot) = self.search.as_ref().and_then(SearchHandle::latest_snapshot)
            && snapshot.generation == self.search_generation
            && self
                .search_snapshot
                .as_ref()
                .is_none_or(|current| !Arc::ptr_eq(current, &snapshot))
        {
            self.search_snapshot = Some(snapshot);
            self.selected_index =
                self.selected_index
                    .min(self.search_snapshot.as_ref().map_or(0, |snapshot| {
                        maximum_visible_result_index(snapshot.results.len())
                    }));
        }
        self.search_error = self
            .storage
            .as_ref()
            .and_then(SearchStorageWorker::last_error)
            .or_else(|| self.extension_search.first_error());
        for result in self.extension_search.take_results() {
            match result.result {
                Ok(()) => self.record_execution(
                    &result.extension_id,
                    &result.entry_id,
                    &result.action_id,
                    &result.query_context,
                ),
                Err(error) => self.action_error = Some(error),
            }
        }
    }

    fn select_previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    fn select_next(&mut self) {
        let maximum = self.search_snapshot.as_ref().map_or(0, |snapshot| {
            maximum_visible_result_index(snapshot.results.len())
        });
        self.selected_index = self.selected_index.saturating_add(1).min(maximum);
    }

    fn poll_runtime(&mut self) {
        let Some(receiver) = &self.runtime_receiver else {
            return;
        };
        let Ok(runtime) = receiver.try_recv() else {
            return;
        };
        self.history = runtime.history;
        self.config = runtime.config;
        self.search_owner = runtime.search_owner;
        self.search = runtime.search;
        self.storage = runtime.storage;
        self.runtime_error = combine_errors(self.runtime_error.take(), runtime.error);
        self.runtime_receiver = None;
        if self.overlay_visible {
            self.begin_search();
        }
    }

    pub fn search_handle(&self) -> Option<SearchHandle> {
        self.search.clone()
    }

    pub fn config_store(&self) -> Option<&ConfigStore> {
        self.config.as_ref()
    }

    pub fn register_search_extension(
        &mut self,
        extension_id: impl Into<String>,
        kind: ExtensionKind,
        process: ExtensionProcess,
    ) -> std::io::Result<()> {
        let extension_id = extension_id.into();
        let search = self
            .search
            .clone()
            .ok_or_else(|| std::io::Error::other("search owner is unavailable"))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| std::io::Error::other("storage owner is unavailable"))?;
        storage
            .register_extension(&extension_id, kind, unix_timestamp())
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        self.extension_search
            .register(extension_id, process, search)?;
        self.begin_search();
        Ok(())
    }

    pub fn record_execution(
        &mut self,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
    ) {
        let executed_at = unix_timestamp();
        let key = UsageKey::new(extension_id, entry_id, action_id, query_context);
        if let Some(storage) = &self.storage
            && let Err(error) = storage.record_usage(
                key.extension_id,
                key.entry_id,
                key.action_id,
                key.query_context,
                executed_at,
            )
        {
            self.operation_error = Some(error.to_string());
        }
    }

    pub fn reset_usage(&mut self) {
        if let Some(storage) = &self.storage
            && let Err(error) = storage.reset_usage()
        {
            self.operation_error = Some(error.to_string());
        }
    }
}

impl Default for HostApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HostApp {
    fn drop(&mut self) {
        GlobalHotKeyEvent::set_event_handler::<fn(GlobalHotKeyEvent)>(None);
        if let Some(thread) = self.runtime_thread.take() {
            let _ = thread.join();
        }
        self.poll_runtime();
        self.extension_search.shutdown();
        if let Some(storage) = self.storage.take() {
            storage.shutdown();
        }
        self.search = None;
        if let Some(owner) = self.search_owner.take() {
            owner.shutdown();
        }
        self.instance.take();
        if let Some(thread) = self.instance_bridge.take() {
            let _ = thread.join();
        }
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
        if self.reveal_on_first_frame {
            self.reveal_on_first_frame = false;
            self.open_overlay(&context);
        }
        self.handle_events(&context);
        self.poll_runtime();
        if !self.search_notifier_configured
            && let Some(search) = &self.search
        {
            let repaint_context = context.clone();
            search.set_notifier(Arc::new(move || repaint_context.request_repaint()));
            let repaint_context = context.clone();
            self.extension_search
                .set_notifier(Arc::new(move || repaint_context.request_repaint()));
            self.search_notifier_configured = true;
        }
        self.refresh_search_snapshot();
        let viewport_focused = context.input(|input| input.viewport().focused);
        if self.overlay_visible && viewport_focused == Some(true) {
            self.focus_observed = true;
        } else if self.overlay_visible
            && self.focus_observed
            && viewport_focused == Some(false)
            && self.motion.target_visible()
        {
            self.close_overlay(&context);
        }

        if !self.overlay_visible && !self.motion.is_active() {
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
                ui.set_opacity(self.motion.value());
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
                    if response.changed() {
                        truncate_chars(&mut self.query, MAX_QUERY_CHARS);
                        self.history.reset_navigation();
                        self.begin_search();
                    }
                    if self.focus_pending {
                        context.memory_mut(|memory| memory.request_focus(input_id));
                        self.focus_pending = false;
                    }

                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        self.submit_query(&context);
                    }
                    if ui.input(|input| {
                        input.modifiers.ctrl && input.key_pressed(egui::Key::ArrowUp)
                    }) {
                        self.navigate_history(HistoryDirection::Older);
                    } else if ui.input(|input| {
                        input.modifiers.ctrl && input.key_pressed(egui::Key::ArrowDown)
                    }) {
                        self.navigate_history(HistoryDirection::Newer);
                    } else if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                        self.select_previous();
                    } else if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                        self.select_next();
                    } else if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        self.close_overlay(&context);
                    }
                    if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Q)) {
                        context.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.add_space(16.0);
                    if let Some(error) = self.hotkey_error.as_ref().or(self.runtime_error.as_ref())
                    {
                        ui.colored_label(egui::Color32::from_rgb(242, 145, 145), error);
                    }
                    if self.extension_search.is_empty() {
                        ui.label(
                            egui::RichText::new("No extensions enabled")
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                    } else if let Some(snapshot) = &self.search_snapshot
                        && snapshot.generation == self.search_generation
                        && !snapshot.results.is_empty()
                    {
                        for (index, result) in snapshot
                            .results
                            .iter()
                            .take(MAX_VISIBLE_RESULTS)
                            .enumerate()
                        {
                            if ui
                                .selectable_label(
                                    index == self.selected_index,
                                    egui::RichText::new(result.candidate.title())
                                        .color(egui::Color32::from_rgb(224, 228, 238)),
                                )
                                .clicked()
                            {
                                self.selected_index = index;
                            }
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("No results")
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                    }
                    if let Some(error) = self
                        .action_error
                        .as_ref()
                        .or(self.operation_error.as_ref())
                        .or(self.search_error.as_ref())
                    {
                        ui.add_space(8.0);
                        ui.colored_label(egui::Color32::from_rgb(242, 145, 145), error);
                    }
                });
            });
    }
}

fn platform_error_message(error: PlatformError) -> String {
    format!("Global hotkey unavailable: {error}")
}

fn combine_errors(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) => Some(format!("{first}; {second}")),
        (Some(error), None) | (None, Some(error)) => Some(error),
        (None, None) => None,
    }
}

fn initialize_search_runtime() -> HostRuntime {
    let mut history_entries = Vec::new();
    let mut usage = UsageMap::new();
    let mut config = None;
    let mut storage = None;
    let mut error = None;

    match NanikaPaths::discover() {
        Some(paths) => {
            match ConfigStore::open(paths.app_data_root(), paths.config_root()) {
                Ok(store) => {
                    if store.is_read_only() {
                        error = combine_errors(
                            error,
                            Some("configuration recovered from backup and is read-only".to_owned()),
                        );
                    }
                    config = Some(store);
                }
                Err(config_error) => {
                    error = combine_errors(error, Some(config_error.to_string()));
                }
            }
            match SearchStorageWorker::spawn(paths.host_database(), 50) {
                Ok((worker, state)) => {
                    history_entries = state.input_history;
                    usage.extend(state.usage.into_iter().map(|stored| {
                        (
                            UsageKey::new(
                                &stored.extension_id,
                                &stored.entry_id,
                                &stored.action_id,
                                &stored.query_context,
                            ),
                            UsageStat {
                                execution_count: stored.execution_count,
                                last_executed_at: stored.last_executed_at,
                            },
                        )
                    }));
                    storage = Some(worker);
                }
                Err(storage_error) => {
                    error = combine_errors(error, Some(storage_error));
                }
            }
        }
        None => error = Some("platform data directories are unavailable".to_owned()),
    }

    let history = InputHistory::from_entries(50, history_entries);
    match SearchOwner::spawn(usage) {
        Ok(owner) => {
            let handle = owner.handle();
            if let Some(storage) = &storage {
                storage.attach_search(handle.clone());
            }
            HostRuntime {
                history,
                config,
                search_owner: Some(owner),
                search: Some(handle),
                storage,
                error,
            }
        }
        Err(search_error) => HostRuntime {
            history,
            config,
            search_owner: None,
            search: None,
            storage,
            error: Some(search_error.to_string()),
        },
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn unix_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

pub(super) fn maximum_visible_result_index(result_count: usize) -> usize {
    result_count.min(MAX_VISIBLE_RESULTS).saturating_sub(1)
}

pub(super) fn truncate_chars(value: &mut String, maximum: usize) {
    let Some(index) = value.char_indices().nth(maximum).map(|(index, _)| index) else {
        return;
    };
    value.truncate(index);
}
