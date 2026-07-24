export interface HardwareMetricsSnapshot {
  timestamp: number;
  cpuUsagePercent: number | null;
  gpuUsagePercent: number | null;
  memoryUsagePercent: number | null;
  cpuTemperatureCelsius: number | null;
  gpuTemperatureCelsius: number | null;
  diskReadBytesPerSecond: number | null;
  diskWriteBytesPerSecond: number | null;
  networkDownloadBytesPerSecond: number | null;
  networkUploadBytesPerSecond: number | null;
  cpuName: string | null;
  gpuName: string | null;
  gpuMemoryUsedBytes: number | null;
  gpuMemoryTotalBytes: number | null;
  totalMemoryBytes: number | null;
  usedMemoryBytes: number | null;
  cpuPhysicalCoreCount: number | null;
  cpuLogicalCoreCount: number | null;
  /** Which probe produced the CPU temperature, for UI precision hints:
   * "librehardwaremonitor" (core MSR, high precision) vs "sysinfo"/"thermalzone"
   * (ACPI estimate). null when no probe succeeded. */
  cpuTemperatureSource: string | null;
  /** Live top-process samples pushed on the `hardware:metrics` event. */
  processes: ProcessUsageSnapshot[];
}

export interface HardwareSnapshot extends HardwareMetricsSnapshot {
  deviceInventory: HardwareDeviceInventory;
}

export interface ProcessUsageSnapshot {
  pid: number;
  name: string;
  cpuUsagePercent: number;
  memoryBytes: number;
  diskReadBytesPerSecond: number | null;
  diskWriteBytesPerSecond: number | null;
}

export interface HardwareDeviceInventory {
  motherboard: HardwareDeviceInfo[];
  memoryModules: MemoryModuleInfo[];
  gpus: HardwareDeviceInfo[];
  displays: HardwareDeviceInfo[];
  disks: HardwareDeviceInfo[];
  audioDevices: HardwareDeviceInfo[];
  networkAdapters: HardwareDeviceInfo[];
}

export interface HardwareDeviceInfo {
  name: string;
  detail: string | null;
  vendor: string | null;
  capacityBytes: number | null;
}

export interface MemoryModuleInfo {
  manufacturer: string | null;
  partNumber: string | null;
  capacityBytes: number | null;
  speedMhz: number | null;
}
