use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use crate::types::{CpuUsage, MemoryUsage};

pub struct UsageCollector {
    system: System,
    collected_information: RefreshKind,
}

impl UsageCollector {
    pub fn new() -> Self {
        let collected_information = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::nothing().with_ram());

        Self {
            system: System::new_with_specifics(collected_information),
            collected_information,
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_specifics(self.collected_information);
    }

    pub fn get_cpu_usage(&self) -> CpuUsage {
        CpuUsage {
            percentage: self.system.global_cpu_usage() as f64,
        }
    }

    pub fn get_memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            total: self.system.total_memory(),
            used: self.system.used_memory(),
            percentage: self.system.used_memory() as f64 / self.system.total_memory() as f64 * 100.,
        }
    }
}
