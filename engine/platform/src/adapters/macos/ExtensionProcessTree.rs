use std::io;

/// Represents the extension process group configured before spawning the child.
pub struct ExtensionProcessTree;

impl ExtensionProcessTree {
    /// The standard child already owns the process group configured before spawning.
    pub fn attach_std(_child: &std::process::Child) -> io::Result<Self> {
        Ok(Self)
    }

    /// The asynchronous child retains the same preconfigured process group.
    pub fn attach_async(_child: &async_process::Child) -> io::Result<Self> {
        Ok(Self)
    }

    /// Terminate the child's process group, accepting only an already absent group.
    pub fn terminate(&self, process_id: u32) -> io::Result<()> {
        if let Some(process_id) = rustix::process::Pid::from_raw(process_id.cast_signed()) {
            handle_process_group_termination(rustix::process::kill_process_group(
                process_id,
                rustix::process::Signal::KILL,
            ))?;
        }
        Ok(())
    }
}

/// Make the extension the leader of a separate process group before it starts.
pub fn configure_extension_command(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

fn handle_process_group_termination(result: rustix::io::Result<()>) -> io::Result<()> {
    match result {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error())),
    }
}

#[cfg(test)]
#[path = "../../../tests/adapters/macos/ExtensionProcessTree.rs"]
mod tests;
