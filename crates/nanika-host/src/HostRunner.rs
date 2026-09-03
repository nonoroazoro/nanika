use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use egui::{ViewportCommand, ViewportId, ViewportInfo};
use egui_wgpu::winit::Painter;
use egui_wgpu::{RendererOptions, SurfaceConfig, WgpuConfiguration};
use egui_winit::{ActionRequested, State};
use nanika_platform::SingleInstance;
use nanika_platform::apply_overlay_visibility;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

use crate::{DesignSystem, HostApp, HostRunnerEvent, OVERLAY_HEIGHT_POINTS, OVERLAY_WIDTH_POINTS};

pub struct HostRunner {
    app: HostApp,
    context: egui::Context,
    proxy: EventLoopProxy<HostRunnerEvent>,
    window: Option<Arc<Window>>,
    input: Option<State>,
    painter: Option<Painter>,
    viewport_info: ViewportInfo,
    actions_requested: Vec<ActionRequested>,
    next_repaint: Option<Instant>,
    visible: bool,
    failure: Option<String>,
}

impl HostRunner {
    pub fn run(instance: SingleInstance, reduced_motion: bool) -> Result<(), String> {
        let event_loop = EventLoop::<HostRunnerEvent>::with_user_event()
            .build()
            .map_err(|error| error.to_string())?;
        let proxy = event_loop.create_proxy();
        let context = egui::Context::default();
        let repaint_proxy = proxy.clone();
        context.set_request_repaint_callback(move |request| {
            if let Some(when) = Instant::now().checked_add(request.delay) {
                let _ = repaint_proxy.send_event(HostRunnerEvent::Repaint {
                    when,
                    cumulative_pass_nr: request.current_cumulative_pass_nr,
                });
            }
        });
        let mut runner = Self {
            app: HostApp::with_instance_and_wake(
                instance,
                reduced_motion,
                Arc::new({
                    let wake_proxy = proxy.clone();
                    move || {
                        let _ = wake_proxy.send_event(HostRunnerEvent::Wake);
                    }
                }),
            ),
            context,
            proxy,
            window: None,
            input: None,
            painter: None,
            viewport_info: ViewportInfo::default(),
            actions_requested: Vec::new(),
            next_repaint: None,
            visible: false,
            failure: None,
        };
        event_loop
            .run_app(&mut runner)
            .map_err(|error| error.to_string())?;
        if let Some(error) = runner.failure {
            Err(error)
        } else {
            Ok(())
        }
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.window.is_some() {
            return Ok(());
        }
        let started_at = Instant::now();
        let window = Arc::new(
            event_loop
                .create_window(overlay_window_attributes())
                .map_err(|error| error.to_string())?,
        );
        let wgpu_configuration =
            WgpuConfiguration::default().with_surface_config(SurfaceConfig::LOW_LATENCY);
        let mut painter = pollster::block_on(Painter::new(
            self.context.clone(),
            wgpu_configuration,
            false,
            RendererOptions::default(),
        ));
        pollster::block_on(painter.set_window(ViewportId::ROOT, Some(Arc::clone(&window))))
            .map_err(|error| error.to_string())?;
        let mut input = State::new(
            self.context.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            event_loop.system_theme(),
            painter.max_texture_side(),
        );
        input.init_accesskit(event_loop, &window, self.proxy.clone());
        egui_winit::update_viewport_info(&mut self.viewport_info, &self.context, &window, true);
        self.window = Some(window);
        self.input = Some(input);
        self.painter = Some(painter);
        self.render(event_loop)?;
        self.apply_native_visibility(false)?;
        tracing::info!(
            elapsed_ms = started_at.elapsed().as_secs_f64() * 1_000.0,
            os_visible = ?self.window.as_ref().and_then(|window| window.is_visible()),
            lifecycle_visible = self.visible,
            "native renderer ready while window remained hidden"
        );
        Ok(())
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let Some(window) = self.window.as_ref().cloned() else {
            return Ok(());
        };

        let render_started_at = Instant::now();
        self.app.mark_activation_render_started(render_started_at);
        egui_winit::update_viewport_info(&mut self.viewport_info, &self.context, &window, false);
        let mut raw_input = self
            .input
            .as_mut()
            .ok_or_else(|| "egui input is not initialized".to_owned())?
            .take_egui_input(&window);
        raw_input
            .viewports
            .insert(ViewportId::ROOT, self.viewport_info.clone());
        let output = self.context.run_ui(raw_input, |ui| {
            self.app.update(ui);
        });
        self.viewport_info.events.clear();
        self.input
            .as_mut()
            .ok_or_else(|| "egui input is not initialized".to_owned())?
            .handle_platform_output_with_event_loop(&window, event_loop, output.platform_output);
        let clipped_primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let mut textures_delta = output.textures_delta;
        self.painter
            .as_mut()
            .ok_or_else(|| "wgpu painter is not initialized".to_owned())?
            .paint_and_update_textures(
                ViewportId::ROOT,
                output.pixels_per_point,
                DesignSystem::clear_color(self.context.theme()),
                &clipped_primitives,
                &mut textures_delta,
                Vec::new(),
                &window,
            );
        self.app.mark_activation_frame_submitted(Instant::now());

        let mut activation_visible_command = false;
        if let Some(viewport) = output.viewport_output.get(&ViewportId::ROOT) {
            let mut commands = Vec::with_capacity(viewport.commands.len());
            for command in &viewport.commands {
                match command {
                    ViewportCommand::Close => event_loop.exit(),
                    ViewportCommand::Visible(visible) => {
                        if self.visible != *visible {
                            tracing::info!(visible, "native window visibility changed");
                        }
                        self.visible = *visible;
                        activation_visible_command |= *visible;
                        let handled = self.apply_native_visibility(*visible)?;
                        if !handled || *visible {
                            commands.push(command.clone());
                        }
                    }
                    _ => commands.push(command.clone()),
                }
            }
            egui_winit::process_viewport_commands(
                &self.context,
                &mut self.viewport_info,
                commands,
                &window,
                &mut self.actions_requested,
            );
        }
        if activation_visible_command {
            self.app
                .mark_activation_visible_command_applied(Instant::now());
        }
        if window.has_focus() {
            self.app.finish_activation(Instant::now());
        }
        self.forward_requested_actions();
        Ok(())
    }

    fn apply_native_visibility(&self, visible: bool) -> Result<bool, String> {
        let Some(window) = self.window.as_ref() else {
            return Ok(false);
        };
        let handle = window.window_handle().map_err(|error| error.to_string())?;
        apply_overlay_visibility(handle.as_raw(), visible).map_err(|error| error.to_string())
    }

    fn forward_requested_actions(&mut self) {
        let Some(input) = self.input.as_mut() else {
            return;
        };
        for action in self.actions_requested.drain(..) {
            let event = match action {
                ActionRequested::Screenshot(_) => None,
                ActionRequested::Cut => Some(egui::Event::Cut),
                ActionRequested::Copy => Some(egui::Event::Copy),
                ActionRequested::Paste => Some(egui::Event::Paste(
                    input.clipboard_text().unwrap_or_default(),
                )),
            };
            if let Some(event) = event {
                input.egui_input_mut().events.push(event);
            }
        }
    }

    fn schedule_repaint(&mut self, event_loop: &ActiveEventLoop, when: Instant) {
        if self.next_repaint.is_none_or(|scheduled| when < scheduled) {
            self.next_repaint = Some(when);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(when));
    }

    fn repaint_now(&mut self, event_loop: &ActiveEventLoop) {
        if self.visible {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else {
            if let Err(error) = self.render(event_loop) {
                self.fail(event_loop, error);
            }
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.failure = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler<HostRunnerEvent> for HostRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.initialize(event_loop) {
            self.fail(event_loop, error);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostRunnerEvent) {
        match event {
            HostRunnerEvent::Wake => {
                self.next_repaint = None;
                self.repaint_now(event_loop);
            }
            HostRunnerEvent::Repaint {
                when,
                cumulative_pass_nr,
            } => {
                let current_pass_nr = self.context.cumulative_pass_nr_for(ViewportId::ROOT);
                if current_pass_nr == cumulative_pass_nr
                    || current_pass_nr == cumulative_pass_nr.saturating_add(1)
                {
                    if when <= Instant::now() {
                        self.repaint_now(event_loop);
                    } else {
                        self.schedule_repaint(event_loop, when);
                    }
                }
            }
            HostRunnerEvent::AccessKit(event) => {
                let Some(input) = self.input.as_mut() else {
                    return;
                };
                match event.window_event {
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        self.context.enable_accesskit();
                    }
                    accesskit_winit::WindowEvent::ActionRequested(request) => {
                        input.on_accesskit_action_request(request);
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                        self.context.disable_accesskit();
                        return;
                    }
                }
                self.repaint_now(event_loop);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }
        if let WindowEvent::Focused(focused) = &event {
            self.app.native_focus_changed(*focused);
            if *focused {
                self.app.finish_activation(Instant::now());
            }
        }
        if matches!(&event, WindowEvent::CloseRequested) {
            self.viewport_info.events.push(egui::ViewportEvent::Close);
        }
        if let WindowEvent::Resized(size) = event
            && let (Some(width), Some(height), Some(painter)) = (
                NonZeroU32::new(size.width),
                NonZeroU32::new(size.height),
                self.painter.as_mut(),
            )
        {
            painter.on_window_resized(ViewportId::ROOT, width, height);
        }
        let repaint = self
            .input
            .as_mut()
            .is_some_and(|input| input.on_window_event(window, &event).repaint);
        if matches!(event, WindowEvent::RedrawRequested) {
            if let Err(error) = self.render(event_loop) {
                self.fail(event_loop, error);
            }
        } else if repaint && self.visible {
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(when) = self.next_repaint else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        if when <= Instant::now() {
            self.next_repaint = None;
            self.repaint_now(event_loop);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(when));
        }
    }
}

fn overlay_window_attributes() -> WindowAttributes {
    let attributes = WindowAttributes::default()
        .with_title("Nanika")
        .with_inner_size(LogicalSize::new(
            OVERLAY_WIDTH_POINTS,
            OVERLAY_HEIGHT_POINTS,
        ))
        .with_min_inner_size(LogicalSize::new(480.0, 120.0))
        .with_decorations(false)
        .with_resizable(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_visible(false);

    #[cfg(windows)]
    {
        use winit::platform::windows::WindowAttributesExtWindows;
        attributes.with_skip_taskbar(true)
    }
    #[cfg(not(windows))]
    {
        attributes
    }
}
