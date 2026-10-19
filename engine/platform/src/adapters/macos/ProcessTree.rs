use std::io;

/// Represents the process group configured before spawning the child.
pub struct ProcessTree;

impl ProcessTree {
    /// The standard child already owns the process group configured before spawning.
    pub fn attach_std(_child: &std::process::Child) -> io::Result<Self> {
        Ok(Self)
    }

    /// Workflow descendants remain owned unless a launch explicitly creates its own group.
    pub fn attach_workflow(child: &std::process::Child) -> io::Result<Self> {
        Self::attach_std(child)
    }

    /// The asynchronous child retains the same preconfigured process group.
    pub fn attach_async(_child: &async_process::Child) -> io::Result<Self> {
        Ok(Self)
    }

    /// Signal zero checks group existence without changing a descendant's state.
    pub fn is_empty(&self, process_id: u32) -> io::Result<bool> {
        let process_id = rustix::process::Pid::from_raw(process_id.cast_signed())
            .ok_or_else(|| io::Error::other("invalid process group"))?;
        match rustix::process::test_kill_process_group(process_id) {
            Ok(()) => Ok(false),
            Err(rustix::io::Errno::SRCH) => Ok(true),
            Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error()))
        }
    }

    /// Ask the owned group to stop before the caller's escalation deadline.
    pub fn request_stop(&self, process_id: u32, interrupt: bool) -> io::Result<()> {
        let pid = rustix::process::Pid::from_raw(process_id.cast_signed())
            .ok_or_else(|| io::Error::other("invalid process group"))?;
        handle_process_group_termination(rustix::process::kill_process_group(
            pid,
            if interrupt {
                rustix::process::Signal::INT
            } else {
                rustix::process::Signal::TERM
            }
        ))
    }

    /// Terminate the child's process group, accepting only an already absent group.
    pub fn terminate(&self, process_id: u32) -> io::Result<()> {
        if let Some(process_id) = rustix::process::Pid::from_raw(process_id.cast_signed()) {
            handle_process_group_termination(rustix::process::kill_process_group(
                process_id,
                rustix::process::Signal::KILL
            ))?;
        }
        Ok(())
    }
}

/// Make the child the leader of a separate process group before it starts.
pub fn configure_process_command(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

fn handle_process_group_termination(result: rustix::io::Result<()>) -> io::Result<()> {
    match result {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error()))
    }
}

#[cfg(test)]
#[path = "../../../tests/adapters/macos/ProcessTree.rs"]
mod tests;
