use std::time::{Duration, Instant};

const SHOW_DURATION: Duration = Duration::from_millis(140);
const HIDE_DURATION: Duration = Duration::from_millis(110);

#[derive(Debug, Default)]
pub(crate) struct OverlayMotion {
    value: f32,
    start_value: f32,
    target: f32,
    started_at: Option<Instant>,
    duration: Duration,
    reduced_motion: bool,
}

impl OverlayMotion {
    pub(crate) fn new(reduced_motion: bool) -> Self {
        Self {
            reduced_motion,
            ..Self::default()
        }
    }

    pub(crate) fn set_target(&mut self, visible: bool) {
        self.set_target_at(visible, Instant::now());
    }

    pub(crate) fn advance(&mut self) -> bool {
        self.advance_at(Instant::now())
    }

    pub(crate) fn value(&self) -> f32 {
        self.value
    }

    pub(crate) fn target_visible(&self) -> bool {
        self.target > 0.5
    }

    pub(crate) fn is_active(&self) -> bool {
        self.started_at.is_some()
    }

    pub(crate) fn set_target_at(&mut self, visible: bool, now: Instant) {
        let target = if visible { 1.0 } else { 0.0 };
        if self.reduced_motion {
            self.value = target;
            self.target = target;
            self.started_at = None;
            return;
        }
        if (self.target - target).abs() < f32::EPSILON {
            return;
        }
        if self.started_at.is_some() {
            self.advance_at(now);
        }
        self.start_value = self.value;
        self.target = target;
        self.duration = if visible {
            SHOW_DURATION
        } else {
            HIDE_DURATION
        };
        self.started_at = Some(now);
    }

    pub(crate) fn advance_at(&mut self, now: Instant) -> bool {
        let Some(started_at) = self.started_at else {
            return false;
        };
        let progress =
            now.saturating_duration_since(started_at).as_secs_f32() / self.duration.as_secs_f32();
        let progress = progress.clamp(0.0, 1.0);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        self.value = self.start_value + (self.target - self.start_value) * eased;
        if progress >= 1.0 {
            self.value = self.target;
            self.started_at = None;
            false
        } else {
            true
        }
    }
}
