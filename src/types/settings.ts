export type MonitorMetric = "Cpu" | "Ram" | "Disk" | "Network" | "Gpu";
export type MonitorBarMode = "Micro" | "Default" | "Expanded";

/** Last successful memory release, surfaced in the settings card. */
export interface LastMemoryRelease {
  timestampMs: number;
  releasedBytes: number;
  fullTier: boolean;
}

export interface AppSettings {
  schemaVersion: number;
  launchAtStartup: boolean;
  isCatVisible: boolean;
  isMonitorBarVisible: boolean;
  enableSleepMode: boolean;
  enableSound: boolean;
  enableNotifications: boolean;
  isProductionPaused: boolean;
  enableLowPowerMode: boolean;
  enableStaticCatMode: boolean;
  enablePetBubble: boolean;
  showMonitorDataInTaskbar: boolean;
  samplingIntervalMs: number;
  backgroundSamplingIntervalMs: number;
  dataSortingCpuThreshold: number;
  catSize: number;
  catOpacity: number;
  catWindowX: number;
  catWindowY: number;
  monitorBarX: number;
  monitorBarY: number;
  cpuTemperatureWarning: number;
  gpuTemperatureWarning: number;
  memoryCrowdedThreshold: number;
  errorGlitchCpuThreshold: number;
  themeName: string;
  visibleMonitorMetrics: MonitorMetric[];
  monitorBarMode: MonitorBarMode;
  visibleTaskbarMetrics: MonitorMetric[];
  taskbarMonitorMode: MonitorBarMode;
  catId: string;
  // Memory release module.
  memoryReleaseEnabled: boolean;
  memoryAutoReleaseEnabled: boolean;
  /** System used-memory threshold in GiB that triggers auto light-tier release. */
  memoryAutoReleaseThresholdGib: number;
  memoryLastRelease: LastMemoryRelease | null;
}

export type AppSettingsPatch = Partial<AppSettings>;
