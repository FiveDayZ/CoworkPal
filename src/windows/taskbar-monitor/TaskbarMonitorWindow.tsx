import { useEffect, useMemo } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useHardwareStore } from "../../stores/hardwareStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { HardwareSnapshot } from "../../types/hardware";
import type { MonitorBarMode, MonitorMetric } from "../../types/settings";
import { getDisplayedMetrics } from "../../services/monitorMetrics";
import { showTaskbarContextMenu } from "../../services/tauriCommands";

const defaultTaskbarMetrics: MonitorMetric[] = ["Cpu", "Ram", "Gpu", "Network"];

type TaskbarMetricCell = {
  key: MonitorMetric;
  top: string;
  bottom: string;
};

export function TaskbarMonitorWindow() {
  const snapshot = useHardwareStore((state) => state.snapshot);
  const settings = useSettingsStore((state) => state.settings);
  const showTaskbarData = settings?.showMonitorDataInTaskbar ?? false;
  const metrics = settings?.visibleTaskbarMetrics ?? defaultTaskbarMetrics;
  const taskbarMode = settings?.taskbarMonitorMode ?? "Default";
  const displayedMetrics = getDisplayedMetrics(metrics, taskbarMode).slice(0, 3);
  const taskbarCells = useMemo(
    () => displayedMetrics.map((metric) => getTaskbarMetricCell(metric, snapshot)),
    [displayedMetrics, snapshot],
  );
  const taskbarText = useMemo(
    () => taskbarCells.map((cell) => `${cell.top} ${cell.bottom}`).join(" "),
    [taskbarCells],
  );

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return undefined;
    }

    const currentWindow = getCurrentWindow();
    let disposed = false;

    async function syncWindow() {
      if (disposed) {
        return;
      }

      if (!showTaskbarData) {
        await currentWindow.hide().catch((err) => {
          console.error("Failed to hide taskbar monitor:", err);
        });
        return;
      }

      await currentWindow.show().catch((err) => {
        console.error("Failed to show taskbar monitor:", err);
      });
      await currentWindow.setTitle(taskbarText).catch((err) => {
        console.error("Failed to update taskbar monitor title:", err);
      });
    }

    void syncWindow();

    return () => {
      disposed = true;
    };
  }, [showTaskbarData, taskbarText]);

  // Right-click shows the native context menu (built on the backend via the
  // same shared menu the tray icon uses). A native popup is required because
  // this window is only ~36px tall and embedded as a WS_CHILD of the shell
  // taskbar — an HTML overlay menu would be clipped by the window rect.
  const handleContextMenu = (event: React.MouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    void showTaskbarContextMenu();
  };

  if (!showTaskbarData) {
    return <div className="cwp-taskbar-monitor-root" />;
  }

  return (
    <div className="cwp-taskbar-monitor-root" onContextMenu={handleContextMenu}>
      <div className="cwp-taskbar-data-text">
        {taskbarCells.length === 0 ? (
          "未选择"
        ) : (
          taskbarCells.map((cell) => (
            <div className="cwp-taskbar-data-cell" key={cell.key}>
              <span>{cell.top}</span>
              <span>{cell.bottom}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function getTaskbarMetricCell(
  metric: MonitorMetric,
  snapshot: HardwareSnapshot | null,
): TaskbarMetricCell {
  if (metric === "Cpu") {
    return {
      key: metric,
      top: `CPU ${formatPercent(snapshot?.cpuUsagePercent ?? null)}`,
      bottom: formatTemperature(snapshot?.cpuTemperatureCelsius ?? null),
    };
  }

  if (metric === "Gpu") {
    return {
      key: metric,
      top: `GPU ${formatPercent(snapshot?.gpuUsagePercent ?? null)}`,
      bottom: formatTemperature(snapshot?.gpuTemperatureCelsius ?? null),
    };
  }

  if (metric === "Ram") {
    return {
      key: metric,
      top: `RAM ${formatPercent(snapshot?.memoryUsagePercent ?? null)}`,
      bottom: formatCompactBytes(snapshot?.usedMemoryBytes ?? null),
    };
  }

  if (metric === "Disk") {
    return {
      key: metric,
      top: `DISK ${formatCompactSpeed(snapshot?.diskReadBytesPerSecond ?? null)}`,
      bottom: `W ${formatCompactSpeed(snapshot?.diskWriteBytesPerSecond ?? null)}`,
    };
  }

  return {
    key: metric,
    top: `NET ${formatCompactSpeed(snapshot?.networkDownloadBytesPerSecond ?? null)}`,
    bottom: `U ${formatCompactSpeed(snapshot?.networkUploadBytesPerSecond ?? null)}`,
  };
}

function formatPercent(value: number | null) {
  if (value == null) {
    return "--%";
  }

  return `${Math.round(Math.max(0, Math.min(100, value)))}%`;
}

function formatTemperature(value: number | null) {
  if (value == null) {
    return "--°C";
  }

  return `${Math.round(value)}°C`;
}

function formatCompactSpeed(value: number | null) {
  if (value == null) {
    return "--";
  }

  return formatCompactBytes(value);
}

function formatCompactBytes(value: number | null) {
  if (value == null) {
    return "--";
  }

  const units = ["B", "K", "M", "G"];
  let size = Math.max(0, value);
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }

  const digits = unitIndex === 0 || size >= 10 ? 0 : 1;
  return `${size.toFixed(digits)}${units[unitIndex]}`;
}
