use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use global_hotkey::GlobalHotKeyEvent;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_platform::{
    HotkeyRegistration, NativeMenu, OverlayPosition, PlatformError, PlatformEvent, SingleInstance,
    StartupService, active_overlay_position,
};
use nanika_protocol::{
    DetailView, ListItem, ListLayout, ListView, NavigationEffect, View, ViewAction,
    ViewActionStyle, ViewEvent,
};
use nanika_search::{
    Candidate, InputHistory, MAX_QUERY_CHARS, SearchHandle, SearchOwner, SearchSnapshot, UsageKey,
    UsageMap, UsageStat, normalize_history_key,
};
use nanika_storage::{ExtensionKind, NanikaPaths, SearchStorageWorker};

use crate::{
    ActivationTrace, DiagnosticCode, ExtensionInvocationOutcome, ExtensionRuntime,
    ExtensionSearchCoordinator, HistoryDirection, HostConfig, HostConfigService, HostDiagnostic,
    HostEvent, HostRuntime, IconIdentity, IconLoader, InvocationPresentation, MAX_VISIBLE_RESULTS,
    OVERLAY_HEIGHT_POINTS, OVERLAY_WIDTH_POINTS, OverlayMotion, PendingHostSettings,
    SettingsAction, SettingsState,
};

const FRAME_INTERVAL: Duration = Duration::from_micros(8_333);
const MAX_EXTENSION_VIEW_DEPTH: usize = 16;
const MAX_ICON_TEXTURES: usize = 256;

fn default_hotkey() -> HotKey {
    #[cfg(target_os = "macos")]
    return HotKey::new(Some(Modifiers::CONTROL), Code::Space);
    #[cfg(windows)]
    return HotKey::new(Some(Modifiers::ALT), Code::Space);
    #[cfg(not(any(windows, target_os = "macos")))]
    HotKey::new(Some(Modifiers::ALT), Code::Space)
}

pub struct HostApp {
    hotkey: Option<HotkeyRegistration>,
    hotkey_error: Option<HostDiagnostic>,
    events: mpsc::Receiver<HostEvent>,
    context_slot: Arc<Mutex<Option<egui::Context>>>,
    instance: Option<SingleInstance>,
    native_menu: Option<NativeMenu>,
    instance_bridge: Option<JoinHandle<()>>,
    runtime_receiver: Option<mpsc::Receiver<HostRuntime>>,
    runtime_thread: Option<JoinHandle<()>>,
    font_receiver: Option<mpsc::Receiver<Option<crate::SystemFont>>>,
    font_thread: Option<JoinHandle<()>>,
    query: String,
    history: InputHistory,
    config: Option<ConfigStore>,
    config_service: Option<HostConfigService>,
    host_config: HostConfig,
    pending_host_settings: Option<PendingHostSettings>,
    startup: Option<StartupService>,
    settings: SettingsState,
    search_owner: Option<SearchOwner>,
    search: Option<SearchHandle>,
    search_notifier_configured: bool,
    search_generation: u64,
    refresh_generation: u64,
    search_snapshot: Option<Arc<SearchSnapshot>>,
    selected_index: usize,
    extension_search: ExtensionSearchCoordinator,
    storage: Option<SearchStorageWorker>,
    icon_loader: Option<IconLoader>,
    icon_textures: HashMap<IconIdentity, (egui::TextureHandle, u64)>,
    pending_icons: HashSet<IconIdentity>,
    failed_icons: HashSet<IconIdentity>,
    icon_access_sequence: u64,
    runtime_errors: Vec<HostDiagnostic>,
    search_error: Option<HostDiagnostic>,
    operation_error: Option<HostDiagnostic>,
    action_error: Option<HostDiagnostic>,
    last_storage_failure_sequence: u64,
    invocation_output: Option<InvocationPresentation>,
    pending_invocation_id: Option<u64>,
    pending_invocation_extension_id: Option<String>,
    view_stack: Vec<crate::ExtensionViewState>,
    pending_view_request_id: Option<u64>,
    overlay_visible: bool,
    focus_pending: bool,
    focus_observed: bool,
    focus_lost_pending: bool,
    motion: OverlayMotion,
    visuals_configured: bool,
    forced_reduced_motion: bool,
    overlay_window_configured: bool,
    last_overlay_position: Option<OverlayPosition>,
    activation_sequence: u64,
    activation_trace: Option<ActivationTrace>,
}

impl HostApp {
    pub fn new() -> Self {
        Self::build(None, false, Arc::new(|| {}))
    }

    pub fn new_with_reduced_motion(reduced_motion: bool) -> Self {
        Self::build(None, reduced_motion, Arc::new(|| {}))
    }

    pub fn with_instance(instance: SingleInstance, reduced_motion: bool) -> Self {
        Self::build(Some(instance), reduced_motion, Arc::new(|| {}))
    }

    pub(crate) fn with_instance_and_wake(
        instance: SingleInstance,
        reduced_motion: bool,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self::build(Some(instance), reduced_motion, wake)
    }

    fn build(
        mut instance: Option<SingleInstance>,
        reduced_motion: bool,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let (sender, events) = mpsc::channel();
        let context_slot: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));
        let hotkey_wake = Arc::clone(&wake);
        let hotkey_events = sender.clone();
        HotkeyRegistration::set_event_handler(move |event: GlobalHotKeyEvent, delivery_delay| {
            let received_at = Instant::now();
            tracing::debug!(
                hotkey_id = event.id,
                hotkey_state = ?event.state,
                delivery_ms = delivery_delay.map(|delay| delay.as_secs_f64() * 1_000.0),
                "native hotkey event"
            );
            let _ = hotkey_events.send(HostEvent::Hotkey {
                event,
                received_at,
                delivery_delay,
            });
            hotkey_wake();
        });

        let (hotkey, hotkey_error) = match HotkeyRegistration::register(default_hotkey()) {
            Ok(hotkey) => (Some(hotkey), None),
            Err(error) => (None, Some(platform_error_message(error))),
        };

        let (native_menu, mut startup_errors) = if let Some(instance) = instance.as_ref() {
            match NativeMenu::new(instance.event_sender()) {
                Ok(menu) => (Some(menu), Vec::new()),
                Err(error) => (
                    None,
                    vec![diagnostic_message(
                        DiagnosticCode::PlatformUnavailable,
                        "initialize native menu",
                        "Nanika could not initialize its tray or menu bar controls.",
                        error.to_string(),
                    )],
                ),
            }
        } else {
            (None, Vec::new())
        };

        let instance_bridge = if let Some(instance) = instance.as_mut() {
            match instance.take_events() {
                Ok(platform_events) => {
                    let host_events = sender;
                    let activation_wake = Arc::clone(&wake);
                    match std::thread::Builder::new()
                        .name("nanika-instance-bridge".to_owned())
                        .spawn(move || {
                            while let Ok(event) = platform_events.recv() {
                                let _ = host_events.send(HostEvent::Platform {
                                    event,
                                    received_at: Instant::now(),
                                });
                                activation_wake();
                            }
                        }) {
                        Ok(thread) => Some(thread),
                        Err(error) => {
                            startup_errors.push(diagnostic_message(
                                DiagnosticCode::InternalFailure,
                                "start instance event bridge",
                                "Nanika could not start its instance event bridge.",
                                error.to_string(),
                            ));
                            None
                        }
                    }
                }
                Err(error) => {
                    startup_errors.push(diagnostic_message(
                        DiagnosticCode::PlatformUnavailable,
                        "take instance events",
                        "Nanika could not receive instance activation events.",
                        error.to_string(),
                    ));
                    None
                }
            }
        } else {
            None
        };

        let (runtime_sender, runtime_receiver) = mpsc::sync_channel(1);
        let runtime_context = Arc::clone(&context_slot);
        let runtime_wake = Arc::clone(&wake);
        let runtime_thread = match std::thread::Builder::new()
            .name("nanika-runtime-initializer".to_owned())
            .spawn(move || {
                let _ = runtime_sender.send(initialize_search_runtime(runtime_wake));
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
                startup_errors.push(diagnostic_message(
                    DiagnosticCode::InternalFailure,
                    "start runtime initializer",
                    "Nanika could not start its runtime initializer.",
                    error.to_string(),
                ));
                None
            }
        };

        let (font_sender, font_receiver) = mpsc::sync_channel(1);
        let font_wake = Arc::clone(&wake);
        let font_thread = match std::thread::Builder::new()
            .name("nanika-system-font-loader".to_owned())
            .spawn(move || {
                let font = crate::load_system_ui_font();
                if let Some(font) = &font {
                    tracing::debug!(
                        family = %font.family,
                        bytes = font.data.len(),
                        "loaded system UI font"
                    );
                } else {
                    tracing::warn!("no system UI font with CJK coverage was found");
                }
                let _ = font_sender.send(font);
                font_wake();
            }) {
            Ok(thread) => Some(thread),
            Err(error) => {
                startup_errors.push(diagnostic_message(
                    DiagnosticCode::InternalFailure,
                    "start system font loader",
                    "Nanika could not prepare fonts for CJK text. Restore the system fonts and restart Nanika.",
                    error.to_string(),
                ));
                None
            }
        };

        let host_config = HostConfig::default();
        Self {
            hotkey,
            hotkey_error,
            events,
            context_slot,
            instance,
            native_menu,
            instance_bridge,
            runtime_receiver: Some(runtime_receiver),
            runtime_thread,
            font_receiver: font_thread.as_ref().map(|_| font_receiver),
            font_thread,
            query: String::new(),
            history: InputHistory::new(50),
            config: None,
            config_service: None,
            settings: SettingsState::new(&host_config),
            host_config,
            pending_host_settings: None,
            startup: None,
            search_owner: None,
            search: None,
            search_notifier_configured: false,
            search_generation: 0,
            refresh_generation: 1,
            search_snapshot: None,
            selected_index: 0,
            extension_search: ExtensionSearchCoordinator::default(),
            storage: None,
            icon_loader: None,
            icon_textures: HashMap::new(),
            pending_icons: HashSet::new(),
            failed_icons: HashSet::new(),
            icon_access_sequence: 0,
            runtime_errors: startup_errors,
            search_error: None,
            operation_error: None,
            action_error: None,
            last_storage_failure_sequence: 0,
            invocation_output: None,
            pending_invocation_id: None,
            pending_invocation_extension_id: None,
            view_stack: Vec::new(),
            pending_view_request_id: None,
            overlay_visible: false,
            focus_pending: false,
            focus_observed: false,
            focus_lost_pending: false,
            motion: OverlayMotion::new(reduced_motion),
            visuals_configured: false,
            forced_reduced_motion: reduced_motion,
            overlay_window_configured: true,
            last_overlay_position: None,
            activation_sequence: 0,
            activation_trace: None,
        }
    }

    fn handle_events(&mut self, context: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                HostEvent::Platform {
                    event: PlatformEvent::Open,
                    received_at,
                } => self.handle_activation("instance", received_at, None, false, context),
                HostEvent::Platform {
                    event: PlatformEvent::Settings,
                    ..
                } => self.open_settings(context),
                HostEvent::Platform {
                    event: PlatformEvent::RescanApplications,
                    ..
                } => {
                    if let Err(error) = self.refresh_applications() {
                        self.operation_error = Some(diagnostic_message(
                            DiagnosticCode::ExtensionUnavailable,
                            "refresh applications",
                            "Application refresh failed. Try again or restart the application extension.",
                            error.to_string(),
                        ));
                    }
                }
                HostEvent::Platform {
                    event: PlatformEvent::Quit,
                    ..
                } => {
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                HostEvent::Platform {
                    event: PlatformEvent::Failure { operation, message },
                    ..
                } => {
                    let diagnostic = diagnostic_message(
                        DiagnosticCode::PlatformUnavailable,
                        operation,
                        "Native platform integration is partially unavailable.",
                        message,
                    );
                    self.runtime_errors
                        .retain(|existing| existing.operation() != operation);
                    self.runtime_errors.push(diagnostic);
                }
                HostEvent::Hotkey {
                    event,
                    received_at,
                    delivery_delay,
                } if self
                    .hotkey
                    .as_ref()
                    .is_some_and(|hotkey| hotkey.is_activation(&event)) =>
                {
                    self.handle_activation("hotkey", received_at, delivery_delay, true, context);
                }
                HostEvent::Hotkey { .. } => {}
            }
        }
    }

    fn handle_activation(
        &mut self,
        source: &'static str,
        received_at: Instant,
        delivery_delay: Option<Duration>,
        toggle: bool,
        context: &egui::Context,
    ) {
        if toggle && self.overlay_visible && self.motion.target_visible() {
            self.close_overlay(context);
            return;
        }
        self.activation_sequence = self.activation_sequence.saturating_add(1);
        let mut trace = ActivationTrace::new(
            self.activation_sequence,
            source,
            received_at,
            delivery_delay,
        );
        trace.mark_handled(Instant::now());
        self.activation_trace = Some(trace);
        self.open_overlay(context);
    }

    pub(crate) fn mark_activation_render_started(&mut self, at: Instant) {
        if let Some(trace) = self.activation_trace.as_mut() {
            trace.mark_render_started(at);
        }
    }

    pub(crate) fn mark_activation_frame_submitted(&mut self, at: Instant) {
        if let Some(trace) = self.activation_trace.as_mut() {
            trace.mark_frame_submitted(at);
        }
    }

    pub(crate) fn mark_activation_visible_command_applied(&mut self, at: Instant) {
        if let Some(trace) = self.activation_trace.as_mut() {
            trace.mark_visible_command_applied(at);
        }
    }

    pub(crate) fn finish_activation(&mut self, at: Instant) {
        if let Some(trace) = self.activation_trace.take() {
            trace.finish(at);
        }
    }

    pub(crate) fn native_focus_changed(&mut self, focused: bool) {
        if focused {
            self.focus_observed = true;
            self.focus_lost_pending = false;
        } else if self.focus_observed {
            self.focus_observed = false;
            self.focus_lost_pending = true;
        }
    }

    fn open_settings(&mut self, context: &egui::Context) {
        self.close_overlay(context);
        self.motion.hide_immediately();
        self.overlay_visible = false;
        self.settings.visible = true;
        self.settings.error = None;
        self.overlay_window_configured = false;
        self.last_overlay_position = None;
        self.request_startup_status();
        context.send_viewport_cmd(egui::ViewportCommand::Title("Nanika Settings".to_owned()));
        context.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(760.0, 680.0)));
        context.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
            560.0, 420.0,
        )));
        context.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
        context.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
        context.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        context.send_viewport_cmd(egui::ViewportCommand::Focus);
        context.request_repaint();
    }

    fn request_startup_status(&mut self) {
        if self.settings.startup_response.is_some() || !self.settings.runtime_ready {
            return;
        }
        let Some(startup) = &self.startup else {
            self.settings.error = Some(diagnostic_notice(
                DiagnosticCode::PlatformUnavailable,
                "query startup status",
                "Startup integration is unavailable. Restart Nanika and try again.",
            ));
            return;
        };
        match startup.query() {
            Ok(response) => self.settings.startup_response = Some(response),
            Err(error) => {
                self.settings.error = Some(diagnostic_message(
                    DiagnosticCode::PlatformUnavailable,
                    "query startup status",
                    "Nanika could not read startup status. Check system startup settings.",
                    error.to_string(),
                ));
            }
        }
    }

    fn open_overlay(&mut self, context: &egui::Context) {
        let interrupting_dismissal = self.overlay_visible && !self.motion.target_visible();
        self.settings.visible = false;
        self.overlay_visible = true;
        self.focus_pending = true;
        self.focus_lost_pending = false;
        if !interrupting_dismissal {
            self.focus_observed = false;
        }
        self.motion.show();
        if !self.overlay_window_configured {
            context.send_viewport_cmd(egui::ViewportCommand::Title("Nanika".to_owned()));
            context.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                OVERLAY_WIDTH_POINTS,
                OVERLAY_HEIGHT_POINTS,
            )));
            context.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                480.0, 240.0,
            )));
            context.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
            context.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
            context.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            self.overlay_window_configured = true;
        }
        if !interrupting_dismissal {
            let native_scale_factor = context
                .native_pixels_per_point()
                .unwrap_or_else(|| context.pixels_per_point());
            if let Some(trace) = self.activation_trace.as_mut() {
                trace.mark_placement_started(Instant::now());
            }
            match active_overlay_position(
                OVERLAY_WIDTH_POINTS,
                OVERLAY_HEIGHT_POINTS,
                native_scale_factor,
            ) {
                Ok(position) => {
                    if self.operation_error.as_ref().is_some_and(|error| {
                        error.user_message()
                            == "Nanika could not place the overlay on the active monitor."
                    }) {
                        self.operation_error = None;
                    }
                    if self.last_overlay_position != Some(position) {
                        let pixels_per_point = context.pixels_per_point();
                        context.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            egui::pos2(
                                position.x / pixels_per_point,
                                position.y / pixels_per_point,
                            ),
                        ));
                        self.last_overlay_position = Some(position);
                    }
                }
                Err(error) => {
                    self.operation_error = Some(diagnostic_message(
                        DiagnosticCode::PlatformUnavailable,
                        "place overlay on active monitor",
                        "Nanika could not place the overlay on the active monitor.",
                        error.to_string(),
                    ));
                }
            }
            if let Some(trace) = self.activation_trace.as_mut() {
                trace.mark_placement_finished(Instant::now());
            }
            context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
        if !interrupting_dismissal || !self.focus_observed {
            context.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if self.pending_invocation_id.is_none() {
            self.invocation_output = None;
            self.begin_search();
        }
        if let Some(trace) = self.activation_trace.as_mut() {
            trace.mark_prepared(Instant::now());
        }
        context.request_repaint();
    }

    fn close_overlay(&mut self, context: &egui::Context) {
        self.activation_trace = None;
        if let (Some(extension_id), Some(invocation_id)) = (
            self.pending_invocation_extension_id.as_deref(),
            self.pending_invocation_id,
        ) && let Err(error) = self
            .extension_search
            .cancel_invocation(extension_id, invocation_id)
        {
            self.action_error = Some(diagnostic_message(
                DiagnosticCode::ExtensionUnavailable,
                "cancel extension invocation",
                "The active extension action could not be cancelled.",
                error.to_string(),
            ));
        }
        for active in self.view_stack.iter().rev() {
            if let Err(error) = self.extension_search.close_view(
                &active.extension_id,
                active.generation,
                active.view_id.clone(),
                active.revision,
            ) {
                tracing::warn!(
                    extension_id = %active.extension_id,
                    view_id = %active.view_id,
                    error = %error,
                    "extension view cleanup could not be queued"
                );
            }
        }
        self.view_stack.clear();
        self.pending_view_request_id = None;
        self.motion.hide();
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

    fn execute_selected(&mut self, context: &egui::Context) {
        self.action_error = None;
        self.invocation_output = None;
        let selected = selected_execution(
            self.search_snapshot.as_deref(),
            self.search_generation,
            self.selected_index,
            &self.query,
        );
        let Some((candidate, query_context)) = selected else {
            self.operation_error = Some(HostDiagnostic::new(
                DiagnosticCode::InternalFailure,
                "resolve selected action",
                "No action is ready.",
            ));
            return;
        };
        let extension_id = candidate.extension_id().to_owned();
        if !query_context.is_empty() {
            self.history.record(&query_context);
            if let Some(storage) = &self.storage
                && let Err(error) = storage.record_history(
                    normalize_history_key(&query_context),
                    query_context.clone(),
                    unix_timestamp_millis(),
                )
            {
                self.operation_error = Some(diagnostic_message(
                    DiagnosticCode::StorageUnavailable,
                    "record input history",
                    "Input history could not be saved.",
                    error.to_string(),
                ));
            }
        }
        match self.extension_search.invoke(
            &extension_id,
            self.search_generation,
            candidate.entry_id(),
            candidate.action_id(),
            query_context,
        ) {
            Ok(invocation_id) => {
                self.pending_invocation_id = Some(invocation_id);
                self.pending_invocation_extension_id = Some(extension_id);
                context.request_repaint();
            }
            Err(error) => {
                self.action_error = Some(diagnostic_message(
                    DiagnosticCode::ExtensionUnavailable,
                    "invoke extension action",
                    "The selected extension action could not start.",
                    error.to_string(),
                ));
            }
        }
    }

    fn apply_navigation_effect(
        &mut self,
        extension_id: String,
        generation: u64,
        effect: NavigationEffect,
        context: &egui::Context,
    ) {
        match effect {
            NavigationEffect::None => {}
            NavigationEffect::Close => self.close_overlay(context),
            NavigationEffect::Pop => {
                self.view_stack.pop();
                self.pending_view_request_id = None;
                self.focus_pending = true;
                context.request_repaint();
            }
            NavigationEffect::Push {
                view_id,
                revision,
                view,
            } => {
                if self
                    .view_stack
                    .iter()
                    .any(|active| active.extension_id == extension_id && active.view_id == view_id)
                {
                    let _ = self.extension_search.close_view(
                        &extension_id,
                        generation,
                        view_id,
                        revision,
                    );
                    self.action_error = Some(diagnostic_notice(
                        DiagnosticCode::ExtensionUnavailable,
                        "push extension view",
                        "The extension reused an active view identifier.",
                    ));
                    return;
                }
                if self.view_stack.len() >= MAX_EXTENSION_VIEW_DEPTH {
                    let _ = self.extension_search.close_view(
                        &extension_id,
                        generation,
                        view_id,
                        revision,
                    );
                    self.action_error = Some(diagnostic_notice(
                        DiagnosticCode::ExtensionUnavailable,
                        "push extension view",
                        "The extension opened too many nested views.",
                    ));
                    return;
                }
                self.invocation_output = None;
                self.action_error = None;
                let search_text = view_search_text(&view);
                self.view_stack.push(crate::ExtensionViewState {
                    extension_id,
                    generation,
                    view_id,
                    revision,
                    view: *view,
                    search_text,
                    queued_search_text: None,
                });
                self.pending_view_request_id = None;
                self.focus_pending = true;
                context.request_repaint();
            }
        }
    }

    fn send_view_event(&mut self, event: ViewEvent, context: &egui::Context) -> bool {
        if self.pending_view_request_id.is_some() {
            return false;
        }
        let Some(active) = self.view_stack.last() else {
            return false;
        };
        match self.extension_search.view_event(
            &active.extension_id,
            active.generation,
            active.view_id.clone(),
            active.revision,
            event,
        ) {
            Ok(request_id) => {
                self.pending_view_request_id = Some(request_id);
                context.request_repaint();
                true
            }
            Err(error) => {
                self.action_error = Some(diagnostic_message(
                    DiagnosticCode::ExtensionUnavailable,
                    "send extension view event",
                    "The current view could not respond.",
                    error.to_string(),
                ));
                false
            }
        }
    }

    fn queue_view_search(&mut self, text: String, context: &egui::Context) {
        let Some(active) = self.view_stack.last_mut() else {
            return;
        };
        active.search_text = Some(text.clone());
        active.queued_search_text = Some(text);
        self.flush_view_search(context);
    }

    fn flush_view_search(&mut self, context: &egui::Context) {
        if self.pending_view_request_id.is_some() {
            return;
        }
        let queued = self
            .view_stack
            .last_mut()
            .and_then(|active| active.queued_search_text.take());
        let Some(text) = queued else {
            return;
        };
        if !self.send_view_event(ViewEvent::SearchChanged { text: text.clone() }, context)
            && let Some(active) = self.view_stack.last_mut()
        {
            active.queued_search_text = Some(text);
        }
    }

    fn close_active_view(&mut self, context: &egui::Context) {
        let Some(active) = self.view_stack.pop() else {
            self.close_overlay(context);
            return;
        };
        if let Err(error) = self.extension_search.close_view(
            &active.extension_id,
            active.generation,
            active.view_id,
            active.revision,
        ) {
            self.action_error = Some(diagnostic_message(
                DiagnosticCode::ExtensionUnavailable,
                "close extension view",
                "The current view could not close.",
                error.to_string(),
            ));
        }
        self.pending_view_request_id = None;
        self.focus_pending = true;
        context.request_repaint();
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
                self.operation_error = Some(diagnostic_message(
                    DiagnosticCode::InternalFailure,
                    "start search query",
                    "Search is temporarily unavailable. Try again.",
                    error.to_string(),
                ));
            }
        }
    }

    fn refresh_search_snapshot(&mut self, context: &egui::Context) {
        if let Some(snapshot) = self.search.as_ref().and_then(SearchHandle::latest_snapshot)
            && snapshot.generation == self.search_generation
            && self
                .search_snapshot
                .as_ref()
                .is_none_or(|current| !Arc::ptr_eq(current, &snapshot))
        {
            self.search_snapshot = Some(snapshot);
            self.failed_icons.clear();
            self.selected_index =
                self.selected_index
                    .min(self.search_snapshot.as_ref().map_or(0, |snapshot| {
                        maximum_visible_result_index(snapshot.results.len())
                    }));
        }
        let storage_diagnostic = self
            .storage
            .as_ref()
            .and_then(SearchStorageWorker::last_failure)
            .map(|failure| {
                let diagnostic = HostDiagnostic::from_message(
                    DiagnosticCode::StorageUnavailable,
                    failure.operation(),
                    "History and usage storage are temporarily unavailable.",
                    failure.source(),
                )
                .with_safe_context("nanika.db");
                if failure.sequence() != self.last_storage_failure_sequence {
                    diagnostic.record_warning();
                    self.last_storage_failure_sequence = failure.sequence();
                }
                diagnostic
            });
        self.search_error = storage_diagnostic.or_else(|| self.extension_search.first_error());
        for output in self.extension_search.take_invocation_outputs() {
            if let Some(presentation) = &mut self.invocation_output {
                presentation.append(output);
            } else {
                self.invocation_output = Some(InvocationPresentation::from_output(output));
            }
        }
        for result in self.extension_search.take_results() {
            let is_pending = self.pending_invocation_id == Some(result.invocation_id);
            if is_pending {
                self.pending_invocation_id = None;
                self.pending_invocation_extension_id = None;
            }
            match result.result {
                Ok(ExtensionInvocationOutcome::Completed { effect, has_output }) => {
                    self.record_execution(
                        &result.extension_id,
                        &result.entry_id,
                        &result.action_id,
                        &result.query_context,
                    );
                    let command_mode = self
                        .extension_search
                        .command_mode(&result.extension_id, &result.entry_id);
                    if is_pending
                        && matches!(
                            command_mode,
                            Some(nanika_extension_package::CommandMode::View)
                        )
                        && !matches!(effect, NavigationEffect::Push { .. })
                    {
                        self.action_error = Some(diagnostic_notice(
                            DiagnosticCode::ExtensionUnavailable,
                            "open extension view",
                            "The selected feature did not provide a view.",
                        ));
                        continue;
                    }
                    if has_output && is_pending {
                        let presentation = self.invocation_output.get_or_insert_with(|| {
                            InvocationPresentation::empty(
                                result.invocation_id,
                                result.extension_id.clone(),
                                result.generation,
                            )
                        });
                        if presentation.invocation_id == result.invocation_id {
                            presentation.complete = true;
                        }
                    }
                    if is_pending {
                        self.apply_navigation_effect(
                            result.extension_id,
                            result.generation,
                            effect,
                            context,
                        );
                    } else if let NavigationEffect::Push {
                        view_id, revision, ..
                    } = effect
                        && let Err(error) = self.extension_search.close_view(
                            &result.extension_id,
                            result.generation,
                            view_id,
                            revision,
                        )
                    {
                        tracing::warn!(
                            extension_id = %result.extension_id,
                            error = %error,
                            "superseded extension view cleanup could not be queued"
                        );
                    }
                }
                Ok(ExtensionInvocationOutcome::Cancelled) => {
                    if let Some(output) = &self.invocation_output
                        && output.invocation_id == result.invocation_id
                    {
                        self.invocation_output = None;
                    }
                }
                Err(error) => {
                    if let Some(output) = &mut self.invocation_output
                        && output.invocation_id == result.invocation_id
                    {
                        output.complete = true;
                    }
                    if is_pending {
                        self.action_error = Some(diagnostic_message(
                            DiagnosticCode::ExtensionUnavailable,
                            "complete extension invocation",
                            "The selected extension action failed.",
                            error,
                        ));
                    }
                }
            }
        }
        for update in self.extension_search.take_view_updates() {
            if self.pending_view_request_id != Some(update.request_id) {
                continue;
            }
            self.pending_view_request_id = None;
            let Some(active) = self.view_stack.last_mut() else {
                continue;
            };
            if active.extension_id != update.extension_id
                || active.generation != update.generation
                || active.view_id != update.view_id
            {
                continue;
            }
            match update.result {
                Ok(payload) => {
                    if let Some(view) = payload.view {
                        active.revision = payload.revision;
                        active.view = view;
                        if active.queued_search_text.is_none() {
                            active.search_text = view_search_text(&active.view);
                        }
                    }
                    self.apply_navigation_effect(
                        update.extension_id,
                        update.generation,
                        payload.effect,
                        context,
                    );
                    self.flush_view_search(context);
                }
                Err(error) => {
                    self.action_error = Some(diagnostic_message(
                        DiagnosticCode::ExtensionUnavailable,
                        "update extension view",
                        "The current view could not be updated.",
                        error,
                    ));
                    self.flush_view_search(context);
                }
            }
        }
        for result in self.extension_search.take_settings() {
            let extension_id = result.extension_id;
            let completed_update = match result.request_id {
                Some(request_id) => {
                    if !self
                        .settings
                        .finish_extension_update(&extension_id, &request_id)
                    {
                        continue;
                    }
                    true
                }
                None => false,
            };
            match result.result {
                Ok(contribution) => {
                    if completed_update {
                        self.settings.dirty.remove(&extension_id);
                    }
                    self.settings.set_contribution(extension_id, contribution);
                }
                Err(error) => {
                    self.settings.error = Some(diagnostic_message(
                        DiagnosticCode::ConfigurationUnavailable,
                        "load extension settings",
                        "Extension settings could not be loaded or saved.",
                        error,
                    ));
                }
            }
        }
        if let Some(response) = &self.settings.startup_response {
            match response.try_recv() {
                Ok(Ok(status)) => {
                    self.settings.startup_status = Some(status);
                    self.settings.startup_response = None;
                }
                Ok(Err(error)) => {
                    self.settings.error = Some(diagnostic_message(
                        DiagnosticCode::PlatformUnavailable,
                        "read startup status",
                        "Nanika could not read startup status. Check system startup settings.",
                        error.to_string(),
                    ));
                    self.settings.startup_response = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.settings.error = Some(diagnostic_notice(
                        DiagnosticCode::PlatformUnavailable,
                        "read startup status",
                        "Startup integration stopped unexpectedly. Restart Nanika and try again.",
                    ));
                    self.settings.startup_response = None;
                }
            }
        }
        self.poll_host_settings();
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
        self.config_service = runtime.config_service;
        self.apply_runtime_host_config(runtime.host_config);
        self.startup = runtime.startup;
        self.settings.runtime_ready = true;
        if self.settings.visible {
            self.request_startup_status();
        }
        self.search_owner = runtime.search_owner;
        self.search = runtime.search;
        self.storage = runtime.storage;
        self.icon_loader = runtime.icon_loader;
        self.runtime_errors.extend(runtime.errors);
        if let Some(host_services) = runtime.host_services {
            self.extension_search.set_host_services(host_services);
        }
        for extension in runtime.pending_extensions {
            if let Err(error) = self.register_search_extension(
                extension.extension_id,
                extension.kind,
                extension.runtime,
                extension.contributions,
            ) {
                self.runtime_errors.push(diagnostic_message(
                    DiagnosticCode::ExtensionUnavailable,
                    "register extension search",
                    "An extension could not join search. Restart Nanika or inspect diagnostics.",
                    error.to_string(),
                ));
            }
        }
        self.runtime_receiver = None;
        if self.overlay_visible {
            self.begin_search();
        }
    }

    fn poll_icon_loader(&mut self, context: &egui::Context) {
        let results = self
            .icon_loader
            .as_ref()
            .map(IconLoader::take_results)
            .unwrap_or_default();
        for result in results {
            self.pending_icons.remove(&result.identity);
            match result.image {
                Ok(image) => {
                    if self.icon_textures.len() >= MAX_ICON_TEXTURES
                        && !self.icon_textures.contains_key(&result.identity)
                        && let Some(oldest) = self
                            .icon_textures
                            .iter()
                            .min_by_key(|(_, (_, last_used))| *last_used)
                            .map(|(identity, _)| identity.clone())
                    {
                        self.icon_textures.remove(&oldest);
                    }
                    self.icon_access_sequence = self.icon_access_sequence.saturating_add(1);
                    let texture = context.load_texture(
                        result.identity.texture_name(),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.icon_textures
                        .insert(result.identity, (texture, self.icon_access_sequence));
                }
                Err(error) => {
                    tracing::warn!(
                        extension_id = %result.identity.extension_id(),
                        icon_key = %result.identity.key(),
                        error = %error,
                        "extension icon could not be loaded"
                    );
                    if self.failed_icons.len() >= MAX_ICON_TEXTURES {
                        self.failed_icons.clear();
                    }
                    self.failed_icons.insert(result.identity);
                }
            }
            context.request_repaint();
        }
    }

    fn poll_system_font(&mut self, context: &egui::Context) {
        let Some(receiver) = &self.font_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(Some(font)) => {
                configure_fonts(context, font);
                self.font_receiver = None;
            }
            Ok(None) => {
                self.runtime_errors.push(diagnostic_notice(
                    DiagnosticCode::PlatformUnavailable,
                    "load CJK system font",
                    "Nanika could not prepare fonts for CJK text. Restore the system fonts and restart Nanika.",
                ));
                self.font_receiver = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.runtime_errors.push(diagnostic_notice(
                    DiagnosticCode::InternalFailure,
                    "load CJK system font",
                    "Nanika could not prepare fonts for CJK text. Restart Nanika and try again.",
                ));
                self.font_receiver = None;
            }
        }
    }

    fn apply_runtime_host_config(&mut self, config: HostConfig) {
        match HotKey::from_str(&config.hotkey) {
            Ok(hotkey) => {
                let result = match &mut self.hotkey {
                    Some(registration) if self.host_config.hotkey != config.hotkey => {
                        registration.replace(hotkey)
                    }
                    Some(_) => Ok(()),
                    None => HotkeyRegistration::register(hotkey).map(|registration| {
                        self.hotkey = Some(registration);
                    }),
                };
                if let Err(error) = result {
                    self.hotkey_error = Some(platform_error_message(error));
                } else {
                    self.hotkey_error = None;
                }
            }
            Err(error) => {
                self.hotkey_error = Some(diagnostic_message(
                    DiagnosticCode::ConfigurationUnavailable,
                    "parse global hotkey",
                    "The configured global hotkey is invalid. Choose another shortcut in Settings.",
                    error.to_string(),
                ));
            }
        }
        self.motion
            .set_reduced_motion(config.reduced_motion || self.forced_reduced_motion);
        self.settings.hotkey = config.hotkey.clone();
        self.settings.reduced_motion = config.reduced_motion;
        self.host_config = config;
    }

    fn poll_host_settings(&mut self) {
        let Some(pending) = &self.pending_host_settings else {
            return;
        };
        match pending.response.try_recv() {
            Ok(Ok(config)) => {
                self.host_config = config;
                self.motion.set_reduced_motion(
                    self.host_config.reduced_motion || self.forced_reduced_motion,
                );
                self.settings.error = None;
                self.settings.saving_host = false;
                self.pending_host_settings = None;
            }
            Ok(Err(error)) => {
                self.rollback_pending_hotkey();
                self.settings.error = Some(diagnostic_message(
                    DiagnosticCode::ConfigurationUnavailable,
                    "persist host settings",
                    "Host settings could not be saved.",
                    error,
                ));
                self.settings.saving_host = false;
                self.pending_host_settings = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.rollback_pending_hotkey();
                self.settings.error = Some(diagnostic_notice(
                    DiagnosticCode::ConfigurationUnavailable,
                    "persist host settings",
                    "The configuration owner stopped unexpectedly. Restart Nanika and try again.",
                ));
                self.settings.saving_host = false;
                self.pending_host_settings = None;
            }
        }
    }

    fn rollback_pending_hotkey(&mut self) {
        let Some(pending) = &self.pending_host_settings else {
            return;
        };
        if pending.new_registration {
            self.hotkey.take();
        } else if let (Some(registration), Some(previous)) =
            (&mut self.hotkey, pending.previous_hotkey)
        {
            let _ = registration.replace(previous);
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
        runtime: ExtensionRuntime,
        contributions: nanika_extension_package::ExtensionContributions,
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
            .register(extension_id, runtime, search, contributions)?;
        self.begin_search();
        Ok(())
    }

    pub fn refresh_applications(&mut self) -> Result<(), crate::SupervisorError> {
        self.refresh_generation = self.refresh_generation.saturating_add(1);
        self.extension_search.refresh(
            crate::builtins::APPLICATION_EXTENSION_ID,
            self.refresh_generation,
        )
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
            self.operation_error = Some(diagnostic_message(
                DiagnosticCode::StorageUnavailable,
                "record action usage",
                "Action usage could not be saved.",
                error.to_string(),
            ));
        }
    }

    pub fn reset_usage(&mut self) {
        if let Some(storage) = &self.storage
            && let Err(error) = storage.reset_usage()
        {
            self.operation_error = Some(diagnostic_message(
                DiagnosticCode::StorageUnavailable,
                "reset action usage",
                "Action usage could not be reset.",
                error.to_string(),
            ));
        }
    }

    fn handle_settings_actions(&mut self, context: &egui::Context, actions: Vec<SettingsAction>) {
        for action in actions {
            match action {
                SettingsAction::SaveHost => self.save_host_settings(),
                SettingsAction::SaveExtension {
                    extension_id,
                    updates,
                } => {
                    if self.settings.pending_extensions.contains_key(&extension_id) {
                        self.settings.error = Some(HostDiagnostic::new(
                            DiagnosticCode::ConfigurationUnavailable,
                            "update extension settings",
                            "An extension settings update is already in progress.",
                        ));
                        continue;
                    }
                    self.refresh_generation = self.refresh_generation.saturating_add(1);
                    let request_id =
                        format!("settings-update-{extension_id}-{}", self.refresh_generation);
                    match self.extension_search.update_settings(
                        &extension_id,
                        request_id.clone(),
                        updates,
                    ) {
                        Ok(()) => {
                            let started = self
                                .settings
                                .begin_extension_update(extension_id, request_id);
                            debug_assert!(started);
                        }
                        Err(error) => {
                            self.settings.error = Some(diagnostic_message(
                                DiagnosticCode::ConfigurationUnavailable,
                                "update extension settings",
                                "Extension settings could not be updated.",
                                error.to_string(),
                            ));
                        }
                    }
                }
                SettingsAction::SetStartup(enabled) => {
                    let Some(startup) = &self.startup else {
                        self.settings.error = Some(diagnostic_notice(
                            DiagnosticCode::PlatformUnavailable,
                            "update startup status",
                            "Startup integration is unavailable. Restart Nanika and try again.",
                        ));
                        continue;
                    };
                    match startup.set_enabled(enabled) {
                        Ok(response) => self.settings.startup_response = Some(response),
                        Err(error) => {
                            self.settings.error = Some(diagnostic_message(
                                DiagnosticCode::PlatformUnavailable,
                                "update startup status",
                                "Nanika could not update startup status. Check system startup settings.",
                                error.to_string(),
                            ));
                        }
                    }
                }
                SettingsAction::Close => {
                    self.settings.visible = false;
                    context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
            }
        }
        if self.settings.startup_response.is_some() || self.pending_host_settings.is_some() {
            context.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn save_host_settings(&mut self) {
        if !self.settings.runtime_ready {
            self.settings.error = Some(HostDiagnostic::new(
                DiagnosticCode::ConfigurationUnavailable,
                "persist host settings",
                "Host runtime is still loading.",
            ));
            return;
        }
        if self.pending_host_settings.is_some() {
            self.settings.error = Some(HostDiagnostic::new(
                DiagnosticCode::ConfigurationUnavailable,
                "persist host settings",
                "A host settings update is already in progress.",
            ));
            return;
        }
        let Some(service) = &self.config_service else {
            self.settings.error = Some(diagnostic_notice(
                DiagnosticCode::ConfigurationUnavailable,
                "persist host settings",
                "Host settings are unavailable. Restart Nanika and try again.",
            ));
            return;
        };
        let replacement = match HotKey::from_str(&self.settings.hotkey) {
            Ok(hotkey) => hotkey,
            Err(error) => {
                self.settings.error = Some(diagnostic_message(
                    DiagnosticCode::ConfigurationUnavailable,
                    "parse global hotkey",
                    "The global hotkey is invalid. Choose another shortcut.",
                    error.to_string(),
                ));
                return;
            }
        };
        let changed_hotkey = self.host_config.hotkey != self.settings.hotkey;
        let previous_hotkey = changed_hotkey
            .then(|| HotKey::from_str(&self.host_config.hotkey).ok())
            .flatten();
        let mut new_registration = false;
        if changed_hotkey {
            let result = match &mut self.hotkey {
                Some(registration) => registration.replace(replacement),
                None => HotkeyRegistration::register(replacement).map(|registration| {
                    self.hotkey = Some(registration);
                    new_registration = true;
                }),
            };
            if let Err(error) = result {
                self.settings.error = Some(platform_error_message(error));
                return;
            }
            self.hotkey_error = None;
        }

        match service.update(self.settings.hotkey.clone(), self.settings.reduced_motion) {
            Ok(response) => {
                self.pending_host_settings = Some(PendingHostSettings {
                    response,
                    previous_hotkey,
                    new_registration,
                });
                self.settings.error = None;
                self.settings.saving_host = true;
            }
            Err(error) => {
                if new_registration {
                    self.hotkey.take();
                } else if let (Some(registration), Some(previous)) =
                    (&mut self.hotkey, previous_hotkey)
                {
                    let _ = registration.replace(previous);
                }
                self.settings.error = Some(diagnostic_message(
                    DiagnosticCode::ConfigurationUnavailable,
                    "queue host settings update",
                    "Host settings could not be queued for saving.",
                    error,
                ));
            }
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
        if let Some(thread) = self.font_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.runtime_thread.take() {
            let _ = thread.join();
        }
        self.poll_runtime();
        self.extension_search.shutdown();
        if let Some(storage) = self.storage.take() {
            storage.shutdown();
        }
        self.icon_loader.take();
        self.search = None;
        if let Some(owner) = self.search_owner.take() {
            owner.shutdown();
        }
        if let Some(service) = self.config_service.take() {
            service.shutdown();
        }
        if let Some(startup) = self.startup.take() {
            startup.shutdown();
        }
        self.native_menu.take();
        self.instance.take();
        if let Some(thread) = self.instance_bridge.take() {
            let _ = thread.join();
        }
    }
}

impl HostApp {
    fn render_extension_view(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let Some(active) = self.view_stack.last().cloned() else {
            return;
        };
        let accepting_actions = self.pending_view_request_id.is_none();
        let mut event = None;
        let mut back_requested = false;
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                back_requested = true;
            }
            ui.add_space(8.0);
            let title = match &active.view {
                View::List { list } => &list.title,
                View::Detail { detail } => detail.title.as_deref().unwrap_or("Detail"),
            };
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(egui::Color32::from_rgb(224, 228, 238)),
            );
            if let View::List { list } = &active.view
                && let Some(filter) = &list.filter
            {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut selected = filter.selected_value.clone();
                    let selected_title = filter
                        .options
                        .iter()
                        .find(|option| option.value == selected)
                        .map_or_else(|| selected.clone(), |option| option.title.clone());
                    ui.add_enabled_ui(accepting_actions, |ui| {
                        egui::ComboBox::from_id_salt((
                            "extension-filter",
                            &active.view_id,
                            &filter.id,
                        ))
                        .selected_text(&selected_title)
                        .show_ui(ui, |ui| {
                            for option in &filter.options {
                                ui.selectable_value(
                                    &mut selected,
                                    option.value.clone(),
                                    &option.title,
                                );
                            }
                        });
                    });
                    if selected != filter.selected_value {
                        event = Some(ViewEvent::FilterChanged {
                            filter_id: filter.id.clone(),
                            value: selected,
                        });
                    }
                });
            }
        });
        if back_requested {
            self.close_active_view(context);
            return;
        }
        ui.add_space(12.0);
        match &active.view {
            View::List { list } => {
                let input_id = egui::Id::new(("extension-query", &active.view_id));
                let mut search_text = active
                    .search_text
                    .clone()
                    .unwrap_or_else(|| list.search_text.clone());
                let response = ui.add(
                    egui::TextEdit::singleline(&mut search_text)
                        .id(input_id)
                        .hint_text(&list.search_placeholder)
                        .font(egui::TextStyle::Heading)
                        .desired_width(f32::INFINITY),
                );
                if response.changed() {
                    truncate_chars(&mut search_text, MAX_QUERY_CHARS);
                    self.queue_view_search(search_text, context);
                }
                if self.focus_pending {
                    context.memory_mut(|memory| memory.request_focus(input_id));
                    self.focus_pending = false;
                }
                let items = list_items(list);
                let selected_index = list
                    .selected_item_id
                    .as_ref()
                    .and_then(|selected| items.iter().position(|item| item.id == *selected));
                if accepting_actions && event.is_none() {
                    if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                        if let Some(item) = previous_list_item(&items, selected_index) {
                            event = Some(ViewEvent::SelectionChanged {
                                item_id: Some(item.id.clone()),
                            });
                        }
                    } else if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                        if let Some(item) = next_list_item(&items, selected_index) {
                            event = Some(ViewEvent::SelectionChanged {
                                item_id: Some(item.id.clone()),
                            });
                        }
                    } else if ui.input(|input| input.key_pressed(egui::Key::Enter))
                        && let Some((item_id, action_id)) = selected_primary_action(list)
                    {
                        event = Some(ViewEvent::ActionInvoked {
                            item_id: Some(item_id),
                            action_id,
                        });
                    }
                }
                ui.add_space(10.0);
                if list.layout == ListLayout::Split {
                    ui.columns(2, |columns| {
                        if let Some(item_id) = render_list(&mut columns[0], list, accepting_actions)
                        {
                            event = Some(ViewEvent::SelectionChanged {
                                item_id: Some(item_id),
                            });
                        }
                        if let Some(detail) = &list.detail {
                            render_detail(&mut columns[1], detail);
                        }
                    });
                } else if let Some(item_id) = render_list(ui, list, accepting_actions) {
                    event = Some(ViewEvent::SelectionChanged {
                        item_id: Some(item_id),
                    });
                }
                if let Some(cursor) = &list.next_cursor
                    && accepting_actions
                    && ui.button("Load more").clicked()
                {
                    event = Some(ViewEvent::LoadMore {
                        cursor: cursor.clone(),
                    });
                }
                if event.is_none()
                    && let Some((item_id, actions)) = selected_actions(list)
                    && let Some(action_id) = render_view_actions(ui, actions, accepting_actions)
                {
                    event = Some(ViewEvent::ActionInvoked {
                        item_id: Some(item_id),
                        action_id,
                    });
                }
            }
            View::Detail { detail } => {
                render_detail(ui, detail);
                if let Some(action_id) = render_view_actions(ui, &detail.actions, accepting_actions)
                {
                    event = Some(ViewEvent::ActionInvoked {
                        item_id: None,
                        action_id,
                    });
                }
            }
        }
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.close_active_view(context);
            return;
        }
        if let Some(error) = self.action_error.as_ref() {
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::from_rgb(242, 145, 145), error.user_message());
        }
        if let Some(event) = event {
            let _ = self.send_view_event(event, context);
        }
    }

    pub(crate) fn update(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        self.poll_system_font(&context);
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
        self.poll_runtime();
        self.poll_icon_loader(&context);
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
        self.refresh_search_snapshot(&context);
        if self.settings.startup_response.is_some() || self.pending_host_settings.is_some() {
            context.request_repaint_after(Duration::from_millis(16));
        }
        let settings_actions = crate::settings_view::show_settings(ui, &mut self.settings);
        self.handle_settings_actions(&context, settings_actions);
        if self.settings.visible {
            return;
        }
        if self.overlay_visible && self.focus_lost_pending && self.motion.target_visible() {
            self.focus_lost_pending = false;
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
                if !self.view_stack.is_empty() {
                    self.render_extension_view(ui, &context);
                    return;
                }
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new("NANIKA")
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::from_rgb(168, 176, 198)),
                    );
                    ui.add_space(12.0);
                    let response = ui
                        .add_enabled_ui(
                            self.pending_invocation_id.is_none()
                                && self.invocation_output.is_none(),
                            |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 54.0],
                                    egui::TextEdit::singleline(&mut self.query)
                                        .id(input_id)
                                        .hint_text("Type a command, app, calculation, or keyword")
                                        .font(egui::TextStyle::Heading),
                                )
                            },
                        )
                        .inner;
                    if response.changed() {
                        truncate_chars(&mut self.query, MAX_QUERY_CHARS);
                        self.history.reset_navigation();
                        self.begin_search();
                    }
                    if self.focus_pending {
                        context.memory_mut(|memory| memory.request_focus(input_id));
                        self.focus_pending = false;
                    }

                    let accepting_input =
                        self.pending_invocation_id.is_none() && self.invocation_output.is_none();
                    if accepting_input {
                        if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                            self.execute_selected(&context);
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
                        }
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        self.close_overlay(&context);
                    }
                    if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Q)) {
                        context.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.add_space(16.0);
                    if let Some(error) = self.hotkey_error.as_ref() {
                        ui.colored_label(
                            egui::Color32::from_rgb(242, 145, 145),
                            error.user_message(),
                        );
                    } else {
                        for (index, error) in self.runtime_errors.iter().enumerate() {
                            if !should_render_runtime_error(&self.runtime_errors, index) {
                                continue;
                            }
                            ui.colored_label(
                                egui::Color32::from_rgb(242, 145, 145),
                                error.user_message(),
                            );
                        }
                    }
                    let mut clicked_result_index = None;
                    if let Some(output) = &self.invocation_output {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&output.extension_id)
                                    .strong()
                                    .color(egui::Color32::from_rgb(168, 176, 198)),
                            );
                            ui.label(
                                egui::RichText::new(if output.complete {
                                    "Complete"
                                } else {
                                    "Running"
                                })
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                            );
                        });
                        ui.add_space(4.0);
                        let (visible_text, truncated) = output.visible_text();
                        if truncated {
                            ui.label(
                                egui::RichText::new("Showing the latest output")
                                    .color(egui::Color32::from_rgb(126, 134, 155)),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(280.0)
                            .show(ui, |ui| {
                                let text = if visible_text.is_empty() && output.complete {
                                    "No output"
                                } else {
                                    visible_text
                                };
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(text)
                                            .color(egui::Color32::from_rgb(224, 228, 238)),
                                    )
                                    .selectable(true)
                                    .wrap(),
                                );
                            });
                    } else if self.pending_invocation_id.is_some() {
                        ui.label(
                            egui::RichText::new("Running...")
                                .color(egui::Color32::from_rgb(168, 176, 198)),
                        );
                    } else if self.extension_search.is_empty() {
                        ui.label(
                            egui::RichText::new(if self.settings.runtime_ready {
                                "No search features are available"
                            } else {
                                "Starting Nanika..."
                            })
                            .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                    } else if let Some(snapshot) = &self.search_snapshot
                        && snapshot.generation == self.search_generation
                        && !snapshot.results.is_empty()
                    {
                        for (index, result, selected) in crate::prepare_visible_results(
                            snapshot,
                            self.search_generation,
                            self.selected_index,
                        ) {
                            let texture_id = icon_texture_id(
                                self.icon_loader.as_ref(),
                                &mut self.icon_textures,
                                &mut self.pending_icons,
                                &self.failed_icons,
                                &mut self.icon_access_sequence,
                                &result.candidate,
                            );
                            let title = egui::RichText::new(result.candidate.title())
                                .color(egui::Color32::from_rgb(224, 228, 238));
                            let button = match texture_id {
                                Some(texture_id) => egui::Button::image_and_text(
                                    (texture_id, egui::vec2(28.0, 28.0)),
                                    title,
                                ),
                                None => egui::Button::new(title),
                            };
                            if ui
                                .add_sized(
                                    [ui.available_width(), 40.0],
                                    button
                                        .selected(selected)
                                        .frame(true)
                                        .frame_when_inactive(selected),
                                )
                                .clicked()
                            {
                                clicked_result_index = Some(index);
                            }
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("No results")
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                    }
                    if let Some(index) = clicked_result_index {
                        self.selected_index = index;
                        self.execute_selected(&context);
                    }
                    if let Some(error) = self
                        .action_error
                        .as_ref()
                        .or(self.operation_error.as_ref())
                        .or(self.search_error.as_ref())
                    {
                        ui.add_space(8.0);
                        ui.colored_label(
                            egui::Color32::from_rgb(242, 145, 145),
                            error.user_message(),
                        );
                    }
                });
            });
    }
}

fn list_items(list: &ListView) -> Vec<&ListItem> {
    list.sections
        .iter()
        .flat_map(|section| section.items.iter())
        .collect()
}

fn previous_list_item<'a>(
    items: &'a [&ListItem],
    selected_index: Option<usize>,
) -> Option<&'a ListItem> {
    let index = selected_index.unwrap_or(0).saturating_sub(1);
    items.get(index).copied()
}

fn next_list_item<'a>(
    items: &'a [&ListItem],
    selected_index: Option<usize>,
) -> Option<&'a ListItem> {
    let index = selected_index.map_or(0, |index| index.saturating_add(1));
    items.get(index.min(items.len().saturating_sub(1))).copied()
}

fn selected_primary_action(list: &ListView) -> Option<(String, String)> {
    let selected = list.selected_item_id.as_deref()?;
    let item = list_items(list)
        .into_iter()
        .find(|item| item.id == selected)?;
    let action = item
        .actions
        .iter()
        .find(|action| action.style == ViewActionStyle::Primary)
        .or_else(|| item.actions.first())?;
    Some((item.id.clone(), action.id.clone()))
}

fn render_list(ui: &mut egui::Ui, list: &ListView, enabled: bool) -> Option<String> {
    let mut clicked = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(330.0)
        .show(ui, |ui| {
            for section in &list.sections {
                if let Some(title) = &section.title {
                    ui.label(
                        egui::RichText::new(title)
                            .strong()
                            .color(egui::Color32::from_rgb(168, 176, 198)),
                    );
                }
                for item in &section.items {
                    let selected = list.selected_item_id.as_deref() == Some(item.id.as_str());
                    let label = match &item.subtitle {
                        Some(subtitle) => format!("{}  {}", item.title, subtitle),
                        None => item.title.clone(),
                    };
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::selectable(
                                selected,
                                egui::RichText::new(label)
                                    .color(egui::Color32::from_rgb(224, 228, 238)),
                            ),
                        )
                        .clicked()
                    {
                        clicked = Some(item.id.clone());
                    }
                }
                ui.add_space(8.0);
            }
        });
    clicked
}

fn selected_actions(list: &ListView) -> Option<(String, &[ViewAction])> {
    let selected = list.selected_item_id.as_deref()?;
    let item = list_items(list)
        .into_iter()
        .find(|item| item.id == selected)?;
    (!item.actions.is_empty()).then(|| (item.id.clone(), item.actions.as_slice()))
}

fn render_view_actions(ui: &mut egui::Ui, actions: &[ViewAction], enabled: bool) -> Option<String> {
    if actions.is_empty() {
        return None;
    }
    ui.separator();
    let mut invoked = None;
    ui.horizontal_wrapped(|ui| {
        for action in actions {
            let button = match action.style {
                ViewActionStyle::Primary => {
                    egui::Button::new(&action.title).fill(egui::Color32::from_rgb(72, 98, 158))
                }
                ViewActionStyle::Secondary => egui::Button::new(&action.title),
                ViewActionStyle::Destructive => egui::Button::new(
                    egui::RichText::new(&action.title)
                        .color(egui::Color32::from_rgb(255, 190, 190)),
                )
                .fill(egui::Color32::from_rgb(105, 46, 50)),
            };
            if ui.add_enabled(enabled, button).clicked() {
                invoked = Some(action.id.clone());
            }
        }
    });
    invoked
}

fn view_search_text(view: &View) -> Option<String> {
    match view {
        View::List { list } => Some(list.search_text.clone()),
        View::Detail { .. } => None,
    }
}

fn render_detail(ui: &mut egui::Ui, detail: &DetailView) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(330.0)
        .show(ui, |ui| {
            if let Some(title) = &detail.title {
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .color(egui::Color32::from_rgb(224, 228, 238)),
                );
                ui.add_space(6.0);
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&detail.body).color(egui::Color32::from_rgb(224, 228, 238)),
                )
                .selectable(true)
                .wrap(),
            );
            if !detail.metadata.is_empty() {
                ui.separator();
                for metadata in &detail.metadata {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&metadata.title)
                                .color(egui::Color32::from_rgb(126, 134, 155)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&metadata.value)
                                    .color(egui::Color32::from_rgb(224, 228, 238)),
                            );
                        });
                    });
                }
            }
        });
}

fn platform_error_message(error: PlatformError) -> HostDiagnostic {
    let diagnostic = HostDiagnostic::from_error(
        DiagnosticCode::PlatformUnavailable,
        "register global hotkey",
        "The global hotkey is unavailable. Choose another shortcut in Settings.",
        error,
    );
    diagnostic.record_warning();
    diagnostic
}

fn diagnostic_message(
    code: DiagnosticCode,
    operation: &'static str,
    user_message: &'static str,
    source: impl Into<String>,
) -> HostDiagnostic {
    let diagnostic = HostDiagnostic::from_message(code, operation, user_message, source);
    diagnostic.record_warning();
    diagnostic
}

fn diagnostic_notice(
    code: DiagnosticCode,
    operation: &'static str,
    user_message: &'static str,
) -> HostDiagnostic {
    let diagnostic = HostDiagnostic::new(code, operation, user_message);
    diagnostic.record_warning();
    diagnostic
}

fn extension_startup_diagnostics(errors: Vec<crate::ExtensionStartupError>) -> Vec<HostDiagnostic> {
    let user_message = extension_startup_user_message(&errors);
    errors
        .into_iter()
        .map(|error| {
            let diagnostic = HostDiagnostic::from_message(
                DiagnosticCode::ExtensionUnavailable,
                "start extension",
                user_message.clone(),
                error.source,
            )
            .with_safe_context(error.diagnostic_context);
            diagnostic.record_warning();
            diagnostic
        })
        .collect()
}

fn initialize_search_runtime(wake: Arc<dyn Fn() + Send + Sync>) -> HostRuntime {
    let mut history_entries = Vec::new();
    let mut usage = UsageMap::new();
    let mut config = None;
    let mut host_config = HostConfig::default();
    let mut extension_registry = ExtensionRegistryConfig::default();
    let mut config_service = None;
    let mut startup = None;
    let mut storage = None;
    let mut icon_loader = None;
    let mut installed_extensions = Vec::new();
    let mut pending_extensions = Vec::new();
    let mut host_services = None;
    let mut errors = Vec::new();

    match NanikaPaths::discover() {
        Some(paths) => {
            match IconLoader::spawn(paths.cache_root(), wake) {
                Ok(loader) => icon_loader = Some(loader),
                Err(error) => errors.push(diagnostic_message(
                    DiagnosticCode::InternalFailure,
                    "start icon loader",
                    "Application icons are temporarily unavailable.",
                    error.to_string(),
                )),
            }
            match ConfigStore::open(paths.app_data_root(), paths.config_root()) {
                Ok(store) => {
                    if store.is_read_only() {
                        errors.push(diagnostic_notice(
                            DiagnosticCode::ConfigurationUnavailable,
                            "recover configuration",
                            "Configuration was recovered from backup and is read-only.",
                        ));
                    }
                    match HostConfig::load(&store) {
                        Ok(loaded) => host_config = loaded,
                        Err(config_error) => {
                            errors.push(diagnostic_message(
                                DiagnosticCode::ConfigurationUnavailable,
                                "load host configuration",
                                "Host configuration could not be loaded. Defaults are active.",
                                config_error,
                            ));
                        }
                    }
                    match ExtensionRegistryConfig::load(&store) {
                        Ok(loaded) => extension_registry = loaded,
                        Err(config_error) => {
                            errors.push(diagnostic_message(
                                DiagnosticCode::ConfigurationUnavailable,
                                "load extension registry",
                                "The extension registry could not be loaded. Built-in defaults are active.",
                                config_error,
                            ));
                        }
                    }
                    match HostConfigService::spawn(store.clone()) {
                        Ok(service) => config_service = Some(service),
                        Err(config_error) => {
                            errors.push(diagnostic_message(
                                DiagnosticCode::ConfigurationUnavailable,
                                "start configuration owner",
                                "Settings cannot be saved because the configuration owner did not start.",
                                config_error.to_string(),
                            ));
                        }
                    }
                    config = Some(store);
                }
                Err(config_error) => {
                    errors.push(diagnostic_message(
                        DiagnosticCode::ConfigurationUnavailable,
                        "open configuration store",
                        "Configuration could not be opened. Check config directory permissions.",
                        config_error.to_string(),
                    ));
                }
            }
            match SearchStorageWorker::spawn(paths.host_database(), 50) {
                Ok((worker, state)) => {
                    history_entries = state.input_history;
                    installed_extensions = state.extensions;
                    for extension_error in state.extension_errors {
                        errors.push(diagnostic_message(
                            DiagnosticCode::StorageUnavailable,
                            "load extension metadata",
                            "Some extension metadata was ignored because it is invalid.",
                            extension_error,
                        ));
                    }
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
                    errors.push(diagnostic_message(
                        DiagnosticCode::StorageUnavailable,
                        "start host storage owner",
                        "History and usage storage are unavailable. Check diagnostics and app-data permissions.",
                        storage_error,
                    ));
                }
            }
            let (extensions, extension_errors) = crate::builtins::spawn_extensions(
                &paths,
                &extension_registry,
                &installed_extensions,
            );
            pending_extensions = extensions;
            errors.extend(extension_startup_diagnostics(extension_errors));
            let (router, service_errors) = crate::HostServiceRouter::spawn(paths.app_data_root());
            for extension in &pending_extensions {
                router.register_permissions(
                    &extension.extension_id,
                    extension.permissions.iter().cloned(),
                );
            }
            host_services = Some(Arc::new(router) as Arc<dyn crate::HostServiceHandler>);
            for service_error in service_errors {
                errors.push(diagnostic_message(
                    DiagnosticCode::InternalFailure,
                    "start host service",
                    "A host service is unavailable. Related extension actions may fail.",
                    service_error,
                ));
            }
            match std::env::current_exe()
                .map_err(|error| error.to_string())
                .and_then(|executable| {
                    StartupService::spawn(executable).map_err(|error| error.to_string())
                }) {
                Ok(service) => startup = Some(service),
                Err(startup_error) => {
                    errors.push(diagnostic_message(
                        DiagnosticCode::PlatformUnavailable,
                        "start startup integration owner",
                        "Startup integration is unavailable. Nanika can still run normally.",
                        startup_error,
                    ));
                }
            }
        }
        None => {
            errors.push(diagnostic_notice(
                DiagnosticCode::PlatformUnavailable,
                "resolve runtime data directories",
                "Runtime data directories are unavailable. Check the current user profile.",
            ));
        }
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
                host_config,
                config_service,
                startup,
                search_owner: Some(owner),
                search: Some(handle),
                storage,
                icon_loader,
                pending_extensions,
                host_services,
                errors,
            }
        }
        Err(search_error) => {
            errors.push(diagnostic_message(
                DiagnosticCode::InternalFailure,
                "start search owner",
                "Search is unavailable because its owner did not start.",
                search_error.to_string(),
            ));
            HostRuntime {
                history,
                config,
                host_config,
                config_service,
                startup,
                search_owner: None,
                search: None,
                storage,
                icon_loader,
                pending_extensions,
                host_services,
                errors,
            }
        }
    }
}

fn configure_fonts(context: &egui::Context, font: crate::SystemFont) {
    context.set_fonts(font.into_font_definitions());
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

pub(super) fn selected_execution(
    snapshot: Option<&SearchSnapshot>,
    generation: u64,
    selected_index: usize,
    query: &str,
) -> Option<(Candidate, String)> {
    snapshot
        .filter(|snapshot| snapshot.generation == generation)
        .and_then(|snapshot| snapshot.results.get(selected_index))
        .map(|result| (result.candidate.clone(), query.trim().to_owned()))
}

fn icon_texture_id(
    loader: Option<&IconLoader>,
    textures: &mut HashMap<IconIdentity, (egui::TextureHandle, u64)>,
    pending: &mut HashSet<IconIdentity>,
    failures: &HashSet<IconIdentity>,
    access_sequence: &mut u64,
    candidate: &Candidate,
) -> Option<egui::TextureId> {
    let identity = IconIdentity::new(candidate.extension_id(), candidate.icon_key()?);
    if let Some((texture, last_used)) = textures.get_mut(&identity) {
        *access_sequence = access_sequence.saturating_add(1);
        *last_used = *access_sequence;
        return Some(texture.id());
    }
    if !pending.contains(&identity)
        && !failures.contains(&identity)
        && loader.is_some_and(|loader| loader.request(identity.clone()).is_ok())
    {
        pending.insert(identity);
    }
    None
}

pub(super) fn extension_startup_user_message(errors: &[crate::ExtensionStartupError]) -> String {
    let mut features = Vec::new();
    for error in errors {
        let feature = match error.diagnostic_context.as_str() {
            crate::builtins::APPLICATION_EXTENSION_ID => "App search",
            crate::builtins::COMMAND_EXTENSION_ID => "commands",
            crate::builtins::SCRIPT_EXTENSION_ID => "scripts",
            crate::builtins::CALCULATOR_EXTENSION_ID => "calculator",
            crate::builtins::CLIPBOARD_EXTENSION_ID => "clipboard history",
            _ => "an installed feature",
        };
        if !features.contains(&feature) {
            features.push(feature);
        }
    }
    let subject = match features.as_slice() {
        [] => "Some Nanika features".to_owned(),
        [feature] => (*feature).to_owned(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let last = features.pop().unwrap_or("an installed feature");
            format!("{}, and {last}", features.join(", "))
        }
    };
    let verb = if features.len() == 1 { "is" } else { "are" };
    format!(
        "{subject} {verb} unavailable. Restart Nanika. If the problem continues, reinstall Nanika or the affected add-on."
    )
}

pub(super) fn should_render_runtime_error(errors: &[HostDiagnostic], index: usize) -> bool {
    let Some(error) = errors.get(index) else {
        return false;
    };
    !errors[..index]
        .iter()
        .any(|existing| existing.user_message() == error.user_message())
}

pub(super) fn truncate_chars(value: &mut String, maximum: usize) {
    let Some(index) = value.char_indices().nth(maximum).map(|(index, _)| index) else {
        return;
    };
    value.truncate(index);
}
