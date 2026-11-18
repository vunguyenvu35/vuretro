use sysinfo::System;
use vuretro_core::model::MachineInfo;
use vuretro_core::ports::Machine;

/// Reads processor, cores and memory of the computer the app runs on.
pub struct SysMachine;

impl Machine for SysMachine {
    fn info(&self) -> MachineInfo {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();
        let cpu_name = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "unknown processor".to_string());
        // logical cores, so a 4-core CPU with hyper-threading counts as 8
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        MachineInfo {
            cpu_name,
            cpu_cores,
            ram_gb: sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0),
            os: System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_computer_reports_something_sensible() {
        let m = SysMachine.info();
        assert!(m.cpu_cores >= 1);
        assert!(m.ram_gb > 0.25, "{m:?}");
        assert!(!m.cpu_name.is_empty() && !m.os.is_empty());
    }
}
