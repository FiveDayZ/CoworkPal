export function formatPercent(value: number | null): string {
  return value == null ? "N/A" : `${value.toFixed(1)}%`;
}

export function formatTemperature(value: number | null): string {
  return value == null ? "N/A" : `${value.toFixed(1)} C`;
}

/** Human-readable label for which probe produced the CPU temperature, used as a
 *  small precision badge next to the reading. Returns an empty string when no
 *  source is reported (older snapshot or no reading at all). */
export function temperatureSourceLabel(source: string | null): string {
  switch (source) {
    case "librehardwaremonitor":
      return "高精度";
    case "sysinfo":
    case "thermalzone":
      return "估算";
    default:
      return "";
  }
}

export function formatBytesPerSecond(value: number | null): string {
  if (value == null) {
    return "N/A";
  }

  return `${formatBytes(value)}/s`;
}

export function formatBytes(value: number | null): string {
  if (value == null) {
    return "N/A";
  }

  const units = ["B", "KB", "MB", "GB"];
  let size = Math.max(0, value);
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }

  return `${size.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

export function formatDuration(seconds: number): string {
  const safeSeconds = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

export function formatParts(value: number): string {
  return value.toFixed(1);
}
