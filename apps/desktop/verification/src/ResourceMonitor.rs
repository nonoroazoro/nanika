use std::collections::HashSet;
use std::time::Instant;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::process_sample::ProcessSample;
use super::resource_snapshot::ResourceSnapshot;

pub struct ResourceMonitor {
    system: System,
    started: Instant
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            started: Instant::now()
        }
    }

    pub fn snapshot(&mut self) -> ResourceSnapshot {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory().with_cpu()
        );
        let mut owned = HashSet::from([Pid::from_u32(std::process::id())]);
        loop {
            let previous = owned.len();
            for (pid, process) in self.system.processes() {
                if process
                    .parent()
                    .is_some_and(|parent| owned.contains(&parent))
                {
                    owned.insert(*pid);
                }
            }
            if owned.len() == previous {
                break;
            }
        }
        let mut processes: Vec<_> = owned
            .into_iter()
            .filter_map(|pid| {
                self.system.process(pid).map(|process| ProcessSample {
                    pid: pid.as_u32(),
                    parent: process.parent().map(|pid| pid.as_u32()),
                    name: process.name().to_string_lossy().into_owned(),
                    resident_bytes: process.memory(),
                    cpu_milliseconds: process.accumulated_cpu_time()
                })
            })
            .collect();
        processes.sort_by_key(|process| process.pid);
        ResourceSnapshot {
            elapsed_ms: self.started.elapsed().as_millis(),
            processes
        }
    }
}
