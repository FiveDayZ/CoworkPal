use crate::{
    models::{
        current_timestamp_ms, HardwareDeviceInfo, HardwareDeviceInventory, HardwareSnapshot,
        MemoryModuleInfo, ProcessUsageSnapshot,
    },
    monitoring::HardwareSensorAdapter,
};

pub struct FakeHardwareSensorAdapter {
    tick: u64,
}

impl FakeHardwareSensorAdapter {
    pub fn new() -> Self {
        Self { tick: 0 }
    }
}

impl HardwareSensorAdapter for FakeHardwareSensorAdapter {
    fn sample(&mut self) -> HardwareSnapshot {
        self.tick = self.tick.wrapping_add(1);
        let wave = (self.tick % 40) as f32;
        let cpu = 20.0 + wave;
        let memory = 45.0 + (wave / 2.0);

        HardwareSnapshot {
            timestamp: current_timestamp_ms(),
            cpu_usage_percent: Some(cpu),
            gpu_usage_percent: None,
            memory_usage_percent: Some(memory),
            cpu_temperature_celsius: None,
            gpu_temperature_celsius: None,
            disk_read_bytes_per_second: Some(64_000.0 + wave * 1000.0),
            disk_write_bytes_per_second: Some(32_000.0 + wave * 750.0),
            network_download_bytes_per_second: Some(120_000.0 + wave * 1500.0),
            network_upload_bytes_per_second: Some(24_000.0 + wave * 500.0),
            cpu_name: Some("Fake CPU".to_string()),
            gpu_name: Some("Fake GPU".to_string()),
            gpu_memory_used_bytes: Some((1.4 * 1024.0 * 1024.0 * 1024.0) as u64),
            gpu_memory_total_bytes: Some(8 * 1024 * 1024 * 1024),
            total_memory_bytes: Some(16 * 1024 * 1024 * 1024),
            used_memory_bytes: Some(((memory / 100.0) * 16.0 * 1024.0 * 1024.0 * 1024.0) as u64),
            cpu_physical_core_count: Some(8),
            cpu_logical_core_count: Some(16),
            cpu_temperature_source: None,
            device_inventory: fake_device_inventory(),
            processes: vec![
                ProcessUsageSnapshot {
                    pid: 3200,
                    name: "Code.exe".to_string(),
                    cpu_usage_percent: 12.0 + wave * 0.3,
                    memory_bytes: 980 * 1024 * 1024,
                    disk_read_bytes_per_second: Some(48_000.0 + wave * 2000.0),
                    disk_write_bytes_per_second: Some(24_000.0 + wave * 1000.0),
                },
                ProcessUsageSnapshot {
                    pid: 4488,
                    name: "chrome.exe".to_string(),
                    cpu_usage_percent: 8.0 + wave * 0.2,
                    memory_bytes: 1536 * 1024 * 1024,
                    disk_read_bytes_per_second: Some(18_000.0),
                    disk_write_bytes_per_second: Some(9_000.0),
                },
                ProcessUsageSnapshot {
                    pid: 6120,
                    name: "node.exe".to_string(),
                    cpu_usage_percent: if self.tick % 8 < 3 { 58.0 } else { 4.0 },
                    memory_bytes: 420 * 1024 * 1024,
                    disk_read_bytes_per_second: Some(256_000.0),
                    disk_write_bytes_per_second: Some(96_000.0),
                },
            ],
        }
    }
}

fn fake_device_inventory() -> HardwareDeviceInventory {
    HardwareDeviceInventory {
        motherboard: vec![HardwareDeviceInfo {
            name: "CoreWorkPal Dev Board".to_string(),
            detail: Some("CW-2026".to_string()),
            vendor: Some("CoreWorkPal".to_string()),
            capacity_bytes: None,
        }],
        memory_modules: vec![
            MemoryModuleInfo {
                manufacturer: Some("CoreWork".to_string()),
                part_number: Some("DDR5-A".to_string()),
                capacity_bytes: Some(8 * 1024 * 1024 * 1024),
                speed_mhz: Some(4800),
            },
            MemoryModuleInfo {
                manufacturer: Some("CoreWork".to_string()),
                part_number: Some("DDR5-B".to_string()),
                capacity_bytes: Some(8 * 1024 * 1024 * 1024),
                speed_mhz: Some(4800),
            },
        ],
        gpus: vec![HardwareDeviceInfo {
            name: "Fake GPU".to_string(),
            detail: Some("Driver 1.0".to_string()),
            vendor: Some("CoreWorkPal".to_string()),
            capacity_bytes: Some(8 * 1024 * 1024 * 1024),
        }],
        displays: vec![HardwareDeviceInfo {
            name: "CoreWorkPal Preview Display".to_string(),
            detail: Some("1920x1080".to_string()),
            vendor: None,
            capacity_bytes: None,
        }],
        disks: vec![HardwareDeviceInfo {
            name: "CoreWorkPal Virtual SSD".to_string(),
            detail: Some("NVMe".to_string()),
            vendor: None,
            capacity_bytes: Some(512 * 1024 * 1024 * 1024),
        }],
        audio_devices: vec![HardwareDeviceInfo {
            name: "CoreWorkPal Audio Device".to_string(),
            detail: Some("OK".to_string()),
            vendor: None,
            capacity_bytes: None,
        }],
        network_adapters: vec![HardwareDeviceInfo {
            name: "CoreWorkPal Network Adapter".to_string(),
            detail: Some("Ethernet".to_string()),
            vendor: None,
            capacity_bytes: None,
        }],
    }
}
