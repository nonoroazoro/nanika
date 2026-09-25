use std::io;

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

use windows_sys::Win32::Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicAccountingInformation, JobObjectExtendedLimitInformation,
    QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

/// Owns the Job Object containing one extension child and its descendants.
pub struct ExtensionProcessTree {
    job: OwnedHandle,
}

impl ExtensionProcessTree {
    /// Attach a child spawned with [`configure_extension_command`] before it can run.
    pub fn attach_std(child: &std::process::Child) -> io::Result<Self> {
        Self::attach_windows(child.as_raw_handle(), child.id())
    }

    /// Attach an asynchronous child before its initial thread is resumed.
    pub fn attach_async(child: &async_process::Child) -> io::Result<Self> {
        Self::attach_windows(child.as_raw_handle(), child.id())
    }

    /// Terminate all processes contained in the owned Job Object.
    pub fn terminate(&self, _process_id: u32) -> io::Result<()> {
        if unsafe { TerminateJobObject(self.job.as_raw_handle().cast(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Observe graceful completion without terminating remaining descendants.
    pub fn is_empty(&self, _process_id: u32) -> io::Result<bool> {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        if unsafe {
            QueryInformationJobObject(
                self.job.as_raw_handle().cast(),
                JobObjectBasicAccountingInformation,
                (&raw mut accounting).cast(),
                u32::try_from(std::mem::size_of_val(&accounting))
                    .expect("accounting size fits u32"),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(accounting.ActiveProcesses == 0)
    }

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

/// Suspend the initial thread so Job assignment precedes any descendant creation.
pub fn configure_extension_command(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};

    command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
}
