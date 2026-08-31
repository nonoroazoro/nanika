use std::time::{Duration, Instant};

const SLOW_ACTIVATION_THRESHOLD: Duration = Duration::from_millis(50);

/// Low-overhead timing state for one overlay activation.
pub(crate) struct ActivationTrace {
    id: u64,
    source: &'static str,
    received_at: Instant,
    delivery_delay: Option<Duration>,
    handled_at: Option<Instant>,
    placement_started_at: Option<Instant>,
    placement_finished_at: Option<Instant>,
    prepared_at: Option<Instant>,
    render_started_at: Option<Instant>,
    frame_submitted_at: Option<Instant>,
    visible_command_applied_at: Option<Instant>,
}

impl ActivationTrace {
    pub(crate) fn new(
        id: u64,
        source: &'static str,
        received_at: Instant,
        delivery_delay: Option<Duration>,
    ) -> Self {
        Self {
            id,
            source,
            received_at,
            delivery_delay,
            handled_at: None,
            placement_started_at: None,
            placement_finished_at: None,
            prepared_at: None,
            render_started_at: None,
            frame_submitted_at: None,
            visible_command_applied_at: None,
        }
    }

    pub(crate) fn mark_handled(&mut self, at: Instant) {
        self.handled_at.get_or_insert(at);
    }

    pub(crate) fn mark_placement_started(&mut self, at: Instant) {
        self.placement_started_at.get_or_insert(at);
    }

    pub(crate) fn mark_placement_finished(&mut self, at: Instant) {
        self.placement_finished_at.get_or_insert(at);
    }

    pub(crate) fn mark_prepared(&mut self, at: Instant) {
        self.prepared_at.get_or_insert(at);
    }

    pub(crate) fn mark_render_started(&mut self, at: Instant) {
        self.render_started_at.get_or_insert(at);
    }

    pub(crate) fn mark_frame_submitted(&mut self, at: Instant) {
        self.frame_submitted_at.get_or_insert(at);
    }

    pub(crate) fn mark_visible_command_applied(&mut self, at: Instant) {
        self.visible_command_applied_at.get_or_insert(at);
    }

    pub(crate) fn finish(self, focused_at: Instant) -> bool {
        let callback_to_focus = focused_at.saturating_duration_since(self.received_at);
        let total = self.delivery_delay.unwrap_or_default() + callback_to_focus;
        let event_queue = elapsed(self.received_at, self.handled_at);
        let placement = elapsed_between(self.placement_started_at, self.placement_finished_at);
        let preparation = elapsed_between(self.handled_at, self.prepared_at);
        let render = elapsed_between(self.render_started_at, self.frame_submitted_at);
        let visibility_command =
            elapsed_between(self.frame_submitted_at, self.visible_command_applied_at);
        let focus_ready = elapsed(
            self.visible_command_applied_at.unwrap_or(focused_at),
            Some(focused_at),
        );
        let slow = total > SLOW_ACTIVATION_THRESHOLD;

        if slow {
            tracing::warn!(
                target: "nanika_perf",
                perf_operation = "overlay_activation",
                activation_id = self.id,
                activation_source = self.source,
                total_ms = total.as_secs_f64() * 1_000.0,
                timing_complete = self.delivery_delay.is_some() || self.source != "hotkey",
                hotkey_delivery_ms = self.delivery_delay.unwrap_or_default().as_secs_f64() * 1_000.0,
                callback_to_focus_ms = callback_to_focus.as_secs_f64() * 1_000.0,
                event_queue_ms = event_queue.as_secs_f64() * 1_000.0,
                preparation_ms = preparation.as_secs_f64() * 1_000.0,
                monitor_placement_ms = placement.as_secs_f64() * 1_000.0,
                render_ms = render.as_secs_f64() * 1_000.0,
                visibility_command_ms = visibility_command.as_secs_f64() * 1_000.0,
                focus_ready_ms = focus_ready.as_secs_f64() * 1_000.0,
                "slow overlay activation"
            );
        } else {
            tracing::debug!(
                target: "nanika_perf",
                perf_operation = "overlay_activation",
                activation_id = self.id,
                activation_source = self.source,
                total_ms = total.as_secs_f64() * 1_000.0,
                timing_complete = self.delivery_delay.is_some() || self.source != "hotkey",
                hotkey_delivery_ms = self.delivery_delay.unwrap_or_default().as_secs_f64() * 1_000.0,
                callback_to_focus_ms = callback_to_focus.as_secs_f64() * 1_000.0,
                event_queue_ms = event_queue.as_secs_f64() * 1_000.0,
                preparation_ms = preparation.as_secs_f64() * 1_000.0,
                monitor_placement_ms = placement.as_secs_f64() * 1_000.0,
                render_ms = render.as_secs_f64() * 1_000.0,
                visibility_command_ms = visibility_command.as_secs_f64() * 1_000.0,
                focus_ready_ms = focus_ready.as_secs_f64() * 1_000.0,
                "overlay activation"
            );
        }

        slow
    }
}

fn elapsed(started_at: Instant, finished_at: Option<Instant>) -> Duration {
    finished_at
        .map(|finished_at| finished_at.saturating_duration_since(started_at))
        .unwrap_or_default()
}

fn elapsed_between(started_at: Option<Instant>, finished_at: Option<Instant>) -> Duration {
    match (started_at, finished_at) {
        (Some(started_at), Some(finished_at)) => finished_at.saturating_duration_since(started_at),
        _ => Duration::default(),
    }
}
