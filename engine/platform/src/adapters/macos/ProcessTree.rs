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

    /// Check for executable members without changing a descendant's state.
    pub fn is_empty(&self, process_id: u32) -> io::Result<bool> {
        let process_id = rustix::process::Pid::from_raw(process_id.cast_signed())
            .ok_or_else(|| io::Error::other("invalid process group"))?;
        match rustix::process::test_kill_process_group(process_id) {
            Ok(()) => Ok(false),
            Err(rustix::io::Errno::SRCH) => Ok(true),
            // Signal-zero denial still establishes that the group exists. A
            // member can be exiting before libproc reports it as a zombie;
            // continue waiting while any member remains, without claiming exit.
            Err(rustix::io::Errno::PERM) => Ok(!_group_has_live_members(process_id.as_raw_pid())?),
            Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error()))
        }
    }

    /// Ask the owned group to stop before the caller's escalation deadline.
    pub fn request_stop(&self, process_id: u32, interrupt: bool) -> io::Result<()> {
        let pid = rustix::process::Pid::from_raw(process_id.cast_signed())
            .ok_or_else(|| io::Error::other("invalid process group"))?;
        _handle_process_group_termination(
            pid.as_raw_pid(),
            rustix::process::kill_process_group(
                pid,
                if interrupt {
                    rustix::process::Signal::INT
                } else {
                    rustix::process::Signal::TERM
                }
            )
        )
    }

    /// Signal the owned group, accepting a denied signal only when no member can run.
    pub fn terminate(&self, process_id: u32) -> io::Result<()> {
        if let Some(process_id) = rustix::process::Pid::from_raw(process_id.cast_signed()) {
            _handle_process_group_termination(
                process_id.as_raw_pid(),
                rustix::process::kill_process_group(process_id, rustix::process::Signal::KILL)
            )?;
        }
        Ok(())
    }
}

/// Isolate the child's process group and inherited descriptors before it starts.
pub fn configure_process_command(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
    // macOS pipe creation and CLOEXEC are separate syscalls. Another thread's
    // half-created pipe must not keep an unrelated extension's output alive.
    // Set flags instead of closing here so Rust can still report exec failures
    // through its private error pipe. Only stdio survives a successful exec.
    unsafe {
        command.pre_exec(|| {
            // close_fds uses allocation-free, async-signal-safe operations here.
            close_fds::set_fds_cloexec(3, &[]);
            Ok(())
        });
    }
}

fn _handle_process_group_termination(
    group: libc::pid_t,
    result: rustix::io::Result<()>
) -> io::Result<()> {
    match result {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(rustix::io::Errno::PERM) if !_group_has_live_members(group)? => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error()))
    }
}

// Darwin returns EPERM for a group containing only unreaped zombies. Permission
// denial alone is never evidence of exit: inspect every member through public libproc.
fn _group_has_live_members(group: libc::pid_t) -> io::Result<bool> {
    let mut members = _group_members(group)?;
    for &pid in &members {
        if _is_live_member(pid, group)? {
            return Ok(true);
        }
    }
    // A parent may fork after enumeration and exit before its status is read.
    // Only a second snapshot with no new members proves the group is quiescent.
    members.sort_unstable();
    Ok(_group_members(group)?
        .iter()
        .any(|pid| members.binary_search(pid).is_err()))
}

fn _group_members(group: libc::pid_t) -> io::Result<Vec<libc::pid_t>> {
    // The sizing call includes kernel headroom. A filled buffer is inconclusive;
    // report that observation failure rather than accepting a partial process tree.
    let capacity = unsafe { libc::proc_listpgrppids(group, std::ptr::null_mut(), 0) };
    if capacity <= 0 {
        return Err(io::Error::last_os_error());
    }
    let bytes = capacity
        .checked_mul(size_of::<libc::pid_t>() as i32)
        .ok_or_else(|| io::Error::other("Process group snapshot exceeds the API buffer limit"))?;
    let mut members = vec![0; capacity as usize];
    // libproc returns zero for both an empty group and errors, leaving errno
    // unchanged on success. Clear this thread's errno before the observation.
    let count = unsafe {
        *libc::__error() = 0;
        libc::proc_listpgrppids(group, members.as_mut_ptr().cast(), bytes)
    };
    if count < 0 || (count == 0 && io::Error::last_os_error().raw_os_error() != Some(0)) {
        return Err(io::Error::last_os_error());
    }
    if count >= capacity {
        return Err(io::Error::other(
            "Process group changed beyond the snapshot capacity"
        ));
    }
    members.truncate(count as usize);
    Ok(members)
}

fn _is_live_member(pid: libc::pid_t, group: libc::pid_t) -> io::Result<bool> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = size_of::<libc::proc_bsdinfo>() as i32;
    // A nonzero argument includes zombies. Read only after libproc fills the
    // complete ABI struct; ESRCH means the member was reaped during observation.
    let count = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            1,
            info.as_mut_ptr().cast(),
            size
        )
    };
    if count == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(false)
        } else {
            Err(error)
        };
    }
    if count != size {
        return Err(io::Error::other(
            "Incomplete process group member information"
        ));
    }
    let info = unsafe { info.assume_init() };
    Ok(info.pbi_pgid == group.cast_unsigned() && info.pbi_status != libc::SZOMB)
}

#[cfg(test)]
#[path = "../../../tests/adapters/macos/ProcessTree.rs"]
mod tests;
