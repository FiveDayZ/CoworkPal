use std::{
    process::Command,
    time::{Duration, Instant},
};

use sysinfo::{Components, Networks, Pid, ProcessRefreshKind, System, UpdateKind};

use crate::{
    models::{
        current_timestamp_ms, HardwareDeviceInventory, HardwareSnapshot, ProcessUsageSnapshot,
    },
    monitoring::HardwareSensorAdapter,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
use super::windows_perf::{
    query_foreground_process_id, query_primary_gpu_info, WindowsGpuInfo, WindowsPerformanceCounters,
};

use super::integrated_hardware_monitor::{
    IntegratedHardwareMonitorProbe, IntegratedHardwareMonitorSample,
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// How long a cached `integrated_hardware_monitor_enabled` reading from settings.json
/// is trusted before the file is re-read. The adapter has no access to the live
/// settings state (its `sample()` takes no args), so it polls the file on disk.
const INTEGRATED_MONITOR_SETTINGS_RECHECK_INTERVAL: Duration = Duration::from_secs(2);

pub struct SysinfoAdapter {
    system: System,
    components: Components,
    networks: Networks,
    last_sample_at: Instant,
    cached_nvidia_sample: Option<NvidiaSmiSample>,
    last_nvidia_query_at: Option<Instant>,
    cached_device_inventory: HardwareDeviceInventory,
    #[cfg(windows)]
    gpu_info: WindowsGpuInfo,
    #[cfg(windows)]
    performance_counters: Option<WindowsPerformanceCounters>,
    integrated_monitor: IntegratedHardwareMonitorProbe,
    /// Whether the user enabled the bundled privileged sensor helper. Re-read
    /// from settings.json so toggling the switch takes effect without restart.
    integrated_monitor_enabled: bool,
    integrated_monitor_settings_checked_at: Option<Instant>,
}

impl SysinfoAdapter {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let components = create_platform_components();
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh();

        Self {
            system,
            components,
            networks,
            last_sample_at: Instant::now(),
            cached_nvidia_sample: None,
            last_nvidia_query_at: None,
            cached_device_inventory: query_device_inventory().unwrap_or_default(),
            #[cfg(windows)]
            gpu_info: query_primary_gpu_info(),
            #[cfg(windows)]
            performance_counters: WindowsPerformanceCounters::new(),
            integrated_monitor: IntegratedHardwareMonitorProbe::new(),
            integrated_monitor_enabled: read_integrated_hardware_monitor_enabled(),
            integrated_monitor_settings_checked_at: Some(Instant::now()),
        }
    }

    fn sample_network_bytes_per_second(
        &mut self,
        elapsed_seconds: f32,
    ) -> (Option<f32>, Option<f32>) {
        self.networks.refresh();

        if self.networks.is_empty() {
            return (None, None);
        }

        let received_bytes = self
            .networks
            .iter()
            .map(|(_, network)| network.received())
            .sum::<u64>() as f32;
        let transmitted_bytes = self
            .networks
            .iter()
            .map(|(_, network)| network.transmitted())
            .sum::<u64>() as f32;
        let elapsed = elapsed_seconds.max(0.001);

        (
            Some(received_bytes / elapsed),
            Some(transmitted_bytes / elapsed),
        )
    }

    fn sample_cpu_temperature(&mut self) -> Option<f32> {
        self.components.refresh();
        select_cpu_component_temperature(
            self.components
                .iter()
                .map(|component| (component.label(), component.temperature())),
        )
    }

    fn sample_nvidia_smi(&mut self) -> Option<NvidiaSmiSample> {
        const NVIDIA_QUERY_INTERVAL: Duration = Duration::from_secs(3);

        let now = Instant::now();
        if self
            .last_nvidia_query_at
            .is_some_and(|previous| now.duration_since(previous) < NVIDIA_QUERY_INTERVAL)
        {
            return self.cached_nvidia_sample.clone();
        }

        self.last_nvidia_query_at = Some(now);
        self.cached_nvidia_sample = query_nvidia_smi();

        self.cached_nvidia_sample.clone()
    }

    fn sample_integrated_hardware_monitor(&mut self) -> Option<IntegratedHardwareMonitorSample> {
        let now = Instant::now();

        // The adapter is intentionally independent of AppState, so it observes
        // the persisted toggle on a short cadence.
        if self
            .integrated_monitor_settings_checked_at
            .is_some_and(|checked| {
                now.duration_since(checked) >= INTEGRATED_MONITOR_SETTINGS_RECHECK_INTERVAL
            })
            || self.integrated_monitor_settings_checked_at.is_none()
        {
            self.integrated_monitor_enabled = read_integrated_hardware_monitor_enabled();
            self.integrated_monitor_settings_checked_at = Some(now);
        }

        self.integrated_monitor
            .sample(self.integrated_monitor_enabled)
    }

    fn sample_processes(
        &mut self,
        elapsed_seconds: f32,
    ) -> (Vec<ProcessUsageSnapshot>, Option<String>) {
        self.system.refresh_processes_specifics(
            ProcessRefreshKind::new()
                .with_cpu()
                .with_memory()
                .with_disk_usage()
                .with_exe(UpdateKind::OnlyIfNotSet),
        );

        #[cfg(windows)]
        let foreground_process_name = query_foreground_process_id().and_then(|process_id| {
            self.system
                .process(Pid::from_u32(process_id))
                .map(|process| process.name().trim().to_string())
                .filter(|name| !name.is_empty())
        });
        #[cfg(not(windows))]
        let foreground_process_name = None;

        let current_pid = std::process::id();
        let elapsed = elapsed_seconds.max(0.001);
        let mut processes = self
            .system
            .processes()
            .iter()
            .filter_map(|(pid, process)| {
                let pid = pid.as_u32();
                if pid == current_pid || pid <= 4 {
                    return None;
                }

                let name = process.name().trim();
                if name.is_empty() || should_skip_process_name(name) {
                    return None;
                }

                let disk_usage = process.disk_usage();
                let disk_read_bytes_per_second =
                    (disk_usage.read_bytes > 0).then_some(disk_usage.read_bytes as f32 / elapsed);
                let disk_write_bytes_per_second = (disk_usage.written_bytes > 0)
                    .then_some(disk_usage.written_bytes as f32 / elapsed);
                let disk_rate = disk_read_bytes_per_second.unwrap_or(0.0)
                    + disk_write_bytes_per_second.unwrap_or(0.0);
                let cpu_usage_percent = process.cpu_usage().max(0.0);
                let memory_bytes = process.memory();

                if cpu_usage_percent < 0.2
                    && memory_bytes < 32 * 1024 * 1024
                    && disk_rate < 8.0 * 1024.0
                {
                    return None;
                }

                Some(ProcessUsageSnapshot {
                    pid,
                    name: name.to_string(),
                    cpu_usage_percent,
                    memory_bytes,
                    disk_read_bytes_per_second,
                    disk_write_bytes_per_second,
                })
            })
            .collect::<Vec<_>>();

        processes.sort_by(|left, right| {
            process_snapshot_score(right)
                .total_cmp(&process_snapshot_score(left))
                .then_with(|| right.memory_bytes.cmp(&left.memory_bytes))
        });
        processes.truncate(32);
        (processes, foreground_process_name)
    }
}

fn create_platform_components() -> Components {
    #[cfg(windows)]
    {
        // sysinfo 0.30 initializes WMI as COM MTA here, while its Windows sensor is an
        // ACPI thermal zone that CoworkPal intentionally rejects as a CPU temperature.
        Components::new()
    }

    #[cfg(not(windows))]
    {
        let mut components = Components::new_with_refreshed_list();
        components.refresh();
        components
    }
}

impl HardwareSensorAdapter for SysinfoAdapter {
    fn sample(&mut self) -> HardwareSnapshot {
        let now = Instant::now();
        let elapsed_seconds = now.duration_since(self.last_sample_at).as_secs_f32();
        self.last_sample_at = now;

        self.system.refresh_cpu();
        self.system.refresh_memory();

        let total_memory_bytes = self.system.total_memory();
        let used_memory_bytes = self.system.used_memory();
        let memory_usage_percent = if total_memory_bytes > 0 {
            Some((used_memory_bytes as f32 / total_memory_bytes as f32) * 100.0)
        } else {
            None
        };

        let cpu_temperature_celsius = self.sample_cpu_temperature();
        let (network_download_bytes_per_second, network_upload_bytes_per_second) =
            self.sample_network_bytes_per_second(elapsed_seconds);
        let nvidia_sample = self.sample_nvidia_smi();
        let (processes, foreground_process_name) = self.sample_processes(elapsed_seconds);

        #[cfg(windows)]
        let performance_sample = self
            .performance_counters
            .as_mut()
            .map(|counters| counters.sample())
            .unwrap_or_default();

        #[cfg(not(windows))]
        let performance_sample = EmptyPerformanceSample::default();

        // Only hardware-scoped sensors are accepted as CPU temperatures. ACPI
        // thermal zones describe a platform/chassis zone and can differ from
        // CPU package temperature by tens of degrees, so they are not used.
        let integrated_sample = self.sample_integrated_hardware_monitor();
        let integrated_cpu_temperature = integrated_sample
            .as_ref()
            .and_then(|sample| sample.cpu_temperature_celsius);
        let (cpu_temperature, cpu_temperature_source) = match integrated_cpu_temperature {
            Some(value) => (Some(value), Some("integrated".to_string())),
            None => match cpu_temperature_celsius {
                Some(value) => (Some(value), Some("sysinfo".to_string())),
                None => (None, None),
            },
        };

        HardwareSnapshot {
            timestamp: current_timestamp_ms(),
            cpu_usage_percent: Some(self.system.global_cpu_info().cpu_usage()),
            gpu_usage_percent: nvidia_sample
                .as_ref()
                .and_then(|sample| sample.usage_percent)
                .or(performance_sample.gpu_usage_percent),
            memory_usage_percent,
            cpu_temperature_celsius: cpu_temperature,
            cpu_temperature_source,
            gpu_temperature_celsius: nvidia_sample
                .as_ref()
                .and_then(|sample| sample.temperature_celsius)
                .or_else(|| {
                    integrated_sample
                        .as_ref()
                        .and_then(|sample| sample.gpu_temperature_celsius)
                }),
            disk_read_bytes_per_second: performance_sample.disk_read_bytes_per_second,
            disk_write_bytes_per_second: performance_sample.disk_write_bytes_per_second,
            network_download_bytes_per_second,
            network_upload_bytes_per_second,
            cpu_name: self
                .system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string())
                .filter(|name| !name.is_empty()),
            gpu_name: nvidia_sample
                .as_ref()
                .and_then(|sample| sample.name.clone())
                .or_else(|| {
                    #[cfg(windows)]
                    {
                        self.gpu_info.name.clone()
                    }
                    #[cfg(not(windows))]
                    {
                        None
                    }
                }),
            gpu_memory_used_bytes: nvidia_sample
                .as_ref()
                .and_then(|sample| sample.memory_used_bytes)
                .or(performance_sample.gpu_memory_used_bytes),
            gpu_memory_total_bytes: nvidia_sample
                .as_ref()
                .and_then(|sample| sample.memory_total_bytes)
                .or_else(|| {
                    #[cfg(windows)]
                    {
                        self.gpu_info.dedicated_memory_bytes
                    }
                    #[cfg(not(windows))]
                    {
                        None
                    }
                }),
            total_memory_bytes: Some(total_memory_bytes),
            used_memory_bytes: Some(used_memory_bytes),
            cpu_physical_core_count: self.system.physical_core_count().map(|count| count as u32),
            cpu_logical_core_count: Some(self.system.cpus().len() as u32),
            device_inventory: self.cached_device_inventory.clone(),
            processes,
            foreground_process_name,
        }
    }
}

fn process_snapshot_score(process: &ProcessUsageSnapshot) -> f32 {
    let disk_rate = process.disk_read_bytes_per_second.unwrap_or(0.0)
        + process.disk_write_bytes_per_second.unwrap_or(0.0);

    process.cpu_usage_percent * 2.0
        + process.memory_bytes as f32 / (128.0 * 1024.0 * 1024.0)
        + disk_rate / (1024.0 * 1024.0)
}

fn select_cpu_component_temperature<'a>(
    components: impl Iterator<Item = (&'a str, f32)>,
) -> Option<f32> {
    let mut best_score = 0;
    let mut values = Vec::new();
    for (label, value) in components {
        if !value.is_finite() || !(0.0..=125.0).contains(&value) {
            continue;
        }
        let label = label.to_ascii_lowercase();
        if label.contains("acpi") || label.contains("thermal zone") {
            continue;
        }
        let score = if label.contains("package") || label.contains("tctl") || label.contains("tdie")
        {
            3
        } else if label.contains("cpu") || label.contains("processor") {
            2
        } else if label.contains("core") {
            1
        } else {
            continue;
        };

        if score > best_score {
            best_score = score;
            values.clear();
        }
        if score == best_score {
            values.push(value);
        }
    }

    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

fn should_skip_process_name(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized.contains("coworkpal")
        || normalized.contains("cowork-pal")
        || normalized == "system idle process"
        || normalized == "idle"
}

#[derive(Debug, Clone, Default)]
struct NvidiaSmiSample {
    name: Option<String>,
    usage_percent: Option<f32>,
    memory_used_bytes: Option<u64>,
    memory_total_bytes: Option<u64>,
    temperature_celsius: Option<f32>,
}

fn query_nvidia_smi() -> Option<NvidiaSmiSample> {
    let mut command = Command::new("nvidia-smi");
    command.args([
        "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
        "--format=csv,noheader,nounits",
    ]);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    // Bounded: a hung nvidia-smi (wedgeed driver) must not hold the hardware
    // adapter lock forever.
    let output = crate::process_util::run_command_with_timeout(
        command,
        crate::process_util::DEFAULT_SUBPROCESS_TIMEOUT,
    )?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    stdout
        .lines()
        .filter_map(parse_nvidia_smi_line)
        .max_by_key(|sample| sample.memory_total_bytes.unwrap_or(0))
}

fn parse_nvidia_smi_line(line: &str) -> Option<NvidiaSmiSample> {
    let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }

    Some(NvidiaSmiSample {
        name: (!parts[0].is_empty()).then(|| parts[0].to_string()),
        usage_percent: parse_f32(parts[1]),
        memory_used_bytes: parse_mib(parts[2]),
        memory_total_bytes: parse_mib(parts[3]),
        temperature_celsius: parse_f32(parts[4]),
    })
}

fn parse_f32(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn parse_mib(value: &str) -> Option<u64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| (value * 1024.0 * 1024.0).round() as u64)
}

#[cfg(windows)]
fn query_device_inventory() -> Option<HardwareDeviceInventory> {
    let script = r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
function Clean($Value) {
  if ($null -eq $Value) { return $null }
  $Text = [string]$Value
  if ([string]::IsNullOrWhiteSpace($Text)) { return $null }
  return $Text.Trim()
}
function Capacity($Value) {
  if ($null -eq $Value) { return $null }
  try {
    $Number = [double]$Value
    if ([double]::IsNaN($Number) -or $Number -le 0) { return $null }
    return [UInt64]$Number
  } catch {
    return $null
  }
}
function Device($Name, $Detail, $Vendor, $CapacityBytes) {
  $CleanName = Clean $Name
  if ($null -eq $CleanName) { return $null }
  return [ordered]@{
    name = $CleanName
    detail = Clean $Detail
    vendor = Clean $Vendor
    capacityBytes = Capacity $CapacityBytes
  }
}
function DecodeMonitorString($Values) {
  if ($null -eq $Values) { return $null }
  $Chars = @()
  foreach ($Value in $Values) {
    if ($Value -eq 0) { break }
    $Chars += [char][int]$Value
  }
  Clean (-join $Chars)
}
function IsVirtualDeviceName($Value) {
  $Text = Clean $Value
  if ($null -eq $Text) { return $false }
  return $Text -match '(?i)virtual|remote|mirror|indirect|idd|oray|gameviewer|parsec|splashtop|spacedesk|dummy|basic render|basic display'
}
# Build a map of GPU name -> accurate dedicated video memory (bytes) from the
# Display class registry. Win32_VideoController.AdapterRAM is a uint32 and is
# truncated for GPUs with >= 4GB VRAM (e.g. an 8GB RTX 4060 Laptop reports
# ~4095MB). The registry value HardwareInformation.qwMemorySize is a uint64 and
# reports the true size, so we prefer it and fall back to AdapterRAM otherwise.
$GpuMemoryByName = @{}
$DisplayClassPath = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}'
if (Test-Path $DisplayClassPath) {
  try {
    foreach ($sub in Get-ChildItem $DisplayClassPath -ErrorAction SilentlyContinue) {
      if ($sub.PSChildName -notmatch '^\d{4}$') { continue }
      $props = Get-ItemProperty -Path $sub.PSPath -ErrorAction SilentlyContinue
      if ($null -eq $props) { continue }
      $desc = Clean $props.DriverDesc
      $qw = $props.'HardwareInformation.qwMemorySize'
      if ($null -ne $desc -and $null -ne $qw) {
        $qwBytes = Capacity $qw
        if ($null -ne $qwBytes -and $qwBytes -gt 0) {
          $GpuMemoryByName[$desc] = $qwBytes
        }
      }
    }
  } catch {}
}
function GpuMemoryBytes($Name, $AdapterRam) {
  $name = Clean $Name
  if ($null -ne $name -and $GpuMemoryByName.ContainsKey($name)) {
    return $GpuMemoryByName[$name]
  }
  return Capacity $AdapterRam
}
$DisplayWmi = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID -ErrorAction SilentlyContinue | Where-Object {
  $_.Active -eq $true -and
  (Clean $_.InstanceName) -like 'DISPLAY\*' -and
  -not (IsVirtualDeviceName $_.InstanceName)
} | ForEach-Object {
  $Name = DecodeMonitorString $_.UserFriendlyName
  if ($null -eq $Name) {
    $Name = DecodeMonitorString $_.ProductCodeID
  }
  $Vendor = DecodeMonitorString $_.ManufacturerName
  Device $Name $_.InstanceName $Vendor $null
} | Where-Object { $_ -ne $null })
$DisplayFallback = @(Get-CimInstance Win32_DesktopMonitor -ErrorAction SilentlyContinue | Where-Object {
  (Clean $_.PNPDeviceID) -like 'DISPLAY\*' -and
  -not (IsVirtualDeviceName $_.Name) -and
  -not (IsVirtualDeviceName $_.PNPDeviceID) -and
  (Clean $_.Name) -notmatch '(?i)^default monitor$'
} | ForEach-Object {
  $Detail = if ($_.ScreenWidth -and $_.ScreenHeight) { "$($_.ScreenWidth)x$($_.ScreenHeight)" } else { $_.PNPDeviceID }
  Device $_.Name $Detail $_.MonitorManufacturer $null
} | Where-Object { $_ -ne $null })
$Inventory = [ordered]@{
  motherboard = @(Get-CimInstance Win32_BaseBoard -ErrorAction SilentlyContinue | ForEach-Object {
    $BoardName = @($_.Manufacturer, $_.Product) | ForEach-Object { Clean $_ } | Where-Object { $_ }
    Device ($BoardName -join ' ') $_.SerialNumber $_.Manufacturer $null
  } | Where-Object { $_ -ne $null })
  memoryModules = @(Get-CimInstance Win32_PhysicalMemory -ErrorAction SilentlyContinue | ForEach-Object {
    [ordered]@{
      manufacturer = Clean $_.Manufacturer
      partNumber = Clean $_.PartNumber
      capacityBytes = Capacity $_.Capacity
      speedMhz = if ($null -eq $_.Speed) { $null } else { [UInt32]$_.Speed }
    }
  })
  gpus = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Where-Object {
    (Clean $_.PNPDeviceID) -like 'PCI\VEN_*' -and
    -not (IsVirtualDeviceName $_.Name) -and
    -not (IsVirtualDeviceName $_.AdapterCompatibility)
  } | ForEach-Object {
    Device $_.Name $_.DriverVersion $_.AdapterCompatibility (GpuMemoryBytes $_.Name $_.AdapterRAM)
  } | Where-Object { $_ -ne $null })
  displays = @(if ($DisplayWmi.Count -gt 0) { $DisplayWmi } else { $DisplayFallback })
  disks = @(Get-CimInstance Win32_DiskDrive -ErrorAction SilentlyContinue | ForEach-Object {
    Device $_.Model $_.InterfaceType $_.Manufacturer $_.Size
  } | Where-Object { $_ -ne $null })
  audioDevices = @(Get-CimInstance Win32_SoundDevice -ErrorAction SilentlyContinue | ForEach-Object {
    Device $_.Name $_.Status $_.Manufacturer $null
  } | Where-Object { $_ -ne $null })
  networkAdapters = @(Get-CimInstance Win32_NetworkAdapter -ErrorAction SilentlyContinue | Where-Object { $_.PhysicalAdapter -eq $true } | ForEach-Object {
    $Detail = if ($_.NetConnectionID) { $_.NetConnectionID } else { $_.AdapterType }
    Device $_.Name $Detail $_.Manufacturer $null
  } | Where-Object { $_ -ne $null })
}
$Inventory | ConvertTo-Json -Depth 5 -Compress
"#;

    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);
    command.creation_flags(CREATE_NO_WINDOW);

    // Bounded: a wedged WMI/CIM provider must not hold the hardware adapter
    // lock forever (this runs in the sampling pump's spawn_blocking thread).
    let output = crate::process_util::run_command_with_timeout(
        command,
        crate::process_util::DEFAULT_SUBPROCESS_TIMEOUT,
    )?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<HardwareDeviceInventory>(stdout.trim()).ok()
}

#[cfg(not(windows))]
fn query_device_inventory() -> Option<HardwareDeviceInventory> {
    Some(HardwareDeviceInventory::default())
}

#[cfg(not(windows))]
#[derive(Default)]
struct EmptyPerformanceSample {
    disk_read_bytes_per_second: Option<f32>,
    disk_write_bytes_per_second: Option<f32>,
    gpu_usage_percent: Option<f32>,
    gpu_memory_used_bytes: Option<u64>,
}

/// Reads the `integratedHardwareMonitorEnabled` flag from settings.json without
/// pulling in the full `AppSettings` type. The adapter polls this on a short
/// cadence so a UI toggle propagates without a restart. Any read/parse failure
/// defaults to false, preventing an unexpected UAC prompt.
fn read_integrated_hardware_monitor_enabled() -> bool {
    let Some(root) = app_data_root() else {
        return false;
    };
    let path = root.join("settings.json");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    value
        .get("integratedHardwareMonitorEnabled")
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false)
}

/// Resolves the per-user app data directory (`%APPDATA%\CoworkPal` on
/// Windows, `$XDG_DATA_HOME/CoworkPal` elsewhere), mirroring
/// `storage::app_data_root` without the cross-module dependency.
fn app_data_root() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(std::path::PathBuf::from)
        .map(|base| base.join("CoworkPal"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_skips_sysinfo_acpi_component_probe() {
        assert!(create_platform_components().is_empty());
    }

    #[test]
    fn cpu_component_selection_ignores_unrelated_hot_components() {
        let components = [
            ("NVMe Composite", 83.0),
            ("ACPI Thermal Zone CPU", 72.0),
            ("Core 0", 50.0),
            ("Core 1", 54.0),
        ];
        assert_eq!(
            select_cpu_component_temperature(components.into_iter()),
            Some(52.0)
        );
    }

    #[test]
    fn cpu_component_selection_prefers_package_sensor() {
        let components = [("Core 0", 50.0), ("Package id 0", 58.0)];
        assert_eq!(
            select_cpu_component_temperature(components.into_iter()),
            Some(58.0)
        );
    }

    #[test]
    fn nvidia_smi_selection_prefers_primary_gpu_memory_size() {
        let output = "NVIDIA A, 10, 100, 4096, 45\nNVIDIA B, 20, 200, 8192, 55";
        let selected = output
            .lines()
            .filter_map(parse_nvidia_smi_line)
            .max_by_key(|sample| sample.memory_total_bytes.unwrap_or(0))
            .unwrap();
        assert_eq!(selected.name.as_deref(), Some("NVIDIA B"));
        assert_eq!(selected.temperature_celsius, Some(55.0));
    }
}
