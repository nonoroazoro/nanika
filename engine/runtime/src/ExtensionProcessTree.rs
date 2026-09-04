use std::io;

#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

/// Platform process-tree ownership for one extension child.
pub(crate) struct ExtensionProcessTree {
    #[cfg(windows)]
    job: OwnedHandle,
}

impl ExtensionProcessTree {
    pub(crate) fn attach_std(child: &std::process::Child) -> io::Result<Self> {
        #[cfg(windows)]
        return Self::attach_windows(child.as_raw_handle(), child.id());

        #[cfg(not(windows))]
        {
            let _ = child;
            Ok(Self {})
        }
    }

    pub(crate) fn attach_async(child: &async_process::Child) -> io::Result<Self> {
        #[cfg(windows)]
        return Self::attach_windows(child.as_raw_handle(), child.id());

        #[cfg(not(windows))]
        {
            let _ = child;
            Ok(Self {})
        }
    }

    pub(crate) fn terminate(&self, _process_id: u32) -> io::Result<()> {
        #[cfg(windows)]
        {
            if unsafe { TerminateJobObject(self.job.as_raw_handle().cast(), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }

        #[cfg(target_os = "macos")]
        if let Some(process_id) = rustix::process::Pid::from_raw(_process_id.cast_signed()) {
            handle_process_group_termination(rustix::process::kill_process_group(
                process_id,
                rustix::process::Signal::KILL,
            ))?;
        }

        #[cfg(not(any(windows, target_os = "macos")))]
        let _ = _process_id;

        Ok(())
    }

    #[cfg(windows)]
    fn attach_windows(process: RawHandle, process_id: u32) -> io::Result<Self> {
        let raw_job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw_job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = unsafe { OwnedHandle::from_raw_handle(raw_job.cast()) };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                job.as_raw_handle().cast(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                u32::try_from(std::mem::size_of_val(&limits))
                    .expect("job limits size must fit in u32"),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe { AssignProcessToJobObject(job.as_raw_handle().cast(), process.cast()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        resume_initial_thread(process_id)?;
        Ok(Self { job })
    }
}

#[cfg(target_os = "macos")]
fn handle_process_group_termination(result: rustix::io::Result<()>) -> io::Result<()> {
    match result {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(error.raw_os_error())),
    }
}

#[cfg(windows)]
fn resume_initial_thread(process_id: u32) -> io::Result<()> {
    let raw_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw_snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw_snapshot.cast()) };
    let mut entry = THREADENTRY32 {
        dwSize: u32::try_from(std::mem::size_of::<THREADENTRY32>())
            .expect("thread entry size must fit in u32"),
        ..THREADENTRY32::default()
    };
    if unsafe { Thread32First(snapshot.as_raw_handle().cast(), &raw mut entry) } == 0 {
        return Err(io::Error::last_os_error());
    }
    loop {
        if entry.th32OwnerProcessID == process_id {
            let raw_thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw_thread.is_null() {
                return Err(io::Error::last_os_error());
            }
            let thread = unsafe { OwnedHandle::from_raw_handle(raw_thread.cast()) };
            match unsafe { ResumeThread(thread.as_raw_handle().cast()) } {
                1 => return Ok(()),
                u32::MAX => return Err(io::Error::last_os_error()),
                suspend_count => {
                    return Err(io::Error::other(format!(
                        "extension initial thread had unexpected suspend count {suspend_count}"
                    )));
                }
            }
        }
        if unsafe { Thread32Next(snapshot.as_raw_handle().cast(), &raw mut entry) } == 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(ERROR_NO_MORE_FILES.cast_signed()) {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "extension initial thread was not found",
                ))
            } else {
                Err(error)
            };
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::handle_process_group_termination;

    #[test]
    fn process_group_termination_ignores_only_a_missing_process() {
        assert!(handle_process_group_termination(Ok(())).is_ok());
        assert!(handle_process_group_termination(Err(rustix::io::Errno::SRCH)).is_ok());
        let error = handle_process_group_termination(Err(rustix::io::Errno::PERM))
            .expect_err("permission errors must be propagated");
        assert_eq!(
            error.raw_os_error(),
            Some(rustix::io::Errno::PERM.raw_os_error())
        );
    }
}
