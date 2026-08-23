use std::time::Instant;

pub(crate) enum HostRunnerEvent {
    Repaint {
        when: Instant,
        cumulative_pass_nr: u64,
    },
    AccessKit(accesskit_winit::Event),
}

impl From<accesskit_winit::Event> for HostRunnerEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::AccessKit(event)
    }
}
