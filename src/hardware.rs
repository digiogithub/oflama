#![allow(dead_code)]
use nvml_wrapper::Nvml;
use sysinfo::System;

pub struct SystemInfo {
    pub total_memory: u64,
    pub used_memory: u64,
    pub cpu_cores: usize,
    pub has_nvidia: bool,
    pub nvidia_vram_total: u64,
    pub nvidia_vram_free: u64,
    pub has_intel_gpu: bool,
}

pub fn get_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let cpu_cores = sys.cpus().len();

    let mut has_nvidia = false;
    let mut nvidia_vram_total = 0;
    let mut nvidia_vram_free = 0;

    if let Ok(nvml) = Nvml::init() {
        if let Ok(count) = nvml.device_count() {
            if count > 0 {
                has_nvidia = true;
                if let Ok(device) = nvml.device_by_index(0) {
                    if let Ok(memory) = device.memory_info() {
                        nvidia_vram_total = memory.total;
                        nvidia_vram_free = memory.free;
                    }
                }
            }
        }
    }

    // A simple heuristic for Intel GPU in Linux is checking /dev/dri/renderD128
    let has_intel_gpu = std::path::Path::new("/dev/dri/renderD128").exists() || std::path::Path::new("/dev/dri/card0").exists();

    SystemInfo {
        total_memory,
        used_memory,
        cpu_cores,
        has_nvidia,
        nvidia_vram_total,
        nvidia_vram_free,
        has_intel_gpu,
    }
}
