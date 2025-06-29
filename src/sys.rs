use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use crate::types::{CpuUsage, MemoryUsage};

pub struct SysInfoCollector {
    system: System,
    collected_information: RefreshKind,
}

impl SysInfoCollector {
    pub fn new() -> Self {
        let collected_information = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram());

        let system = System::new_with_specifics(collected_information);
        Self {
            system,
            collected_information,
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_specifics(self.collected_information);
    }

    pub fn get_cpu_usage(&self) -> CpuUsage {
        CpuUsage {
            usage: self.system.global_cpu_usage(),
        }
    }

    pub fn get_memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            total: self.system.total_memory() as u64,
            used: self.system.used_memory() as u64,
        }
    }
}
