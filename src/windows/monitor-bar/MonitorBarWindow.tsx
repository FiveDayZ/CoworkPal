import type React from "react";
import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { saveWindowPosition } from "../../services/tauriCommands";
import {
  openMainWindow,
  startDraggingCurrentWindow,
} from "../../services/windowApi";
import { useHardwareStore } from "../../stores/hardwareStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { MonitorBarMode, MonitorMetric } from "../../types/settings";
import { PetAvatar, MonitorChip } from "../../ui/components";
import { getMonitorMetricView } from "../../ui/metrics";
import { getDisplayedMetrics } from "../../services/monitorMetrics";

const defaultMetrics: MonitorMetric[] = ["Cpu", "Ram", "Gpu", "Network"];

export function MonitorBarWindow() {
  const snapshot = useHardwareStore((state) => state.snapshot);
  const settings = useSettingsStore((state) => state.settings);
  const metrics = settings?.visibleMonitorMetrics ?? defaultMetrics;
  const monitorBarMode = settings?.monitorBarMode ?? "Default";
  const displayedMetrics = getDisplayedMetrics(metrics, monitorBarMode);
  const dragCandidateRef = useRef(false);
  const saveTimerRef = useRef<number | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    if (!("__TAURI_INTERNALS__" in window)) {
      return undefined;
    }

    const currentWindow = getCurrentWindow();

    void currentWindow.onMoved((event) => {
      if (!dragCandidateRef.current) {
        return;
      }

      if (saveTimerRef.current != null) {
        window.clearTimeout(saveTimerRef.current);
      }

      saveTimerRef.current = window.setTimeout(() => {
        void saveWindowPosition("monitor-bar", event.payload.x, event.payload.y);
      }, 250);
    }).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
      } else {
        unlisten = nextUnlisten;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
      if (saveTimerRef.current != null) {
        window.clearTimeout(saveTimerRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return;
    }

    let disposed = false;
    const currentWindow = getCurrentWindow();
    const handleResize = () => {
      if (disposed) return;
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();

      if (rect.width > 0 && rect.height > 0) {
        const targetWidth = Math.ceil(rect.width);
        const targetHeight = Math.ceil(rect.height);

        currentWindow.setSize(new LogicalSize(targetWidth, targetHeight)).catch((err) => {
          console.error("Failed to resize monitor-bar window:", err);
        });
      }
    };

    const resizeObserver =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(() => handleResize())
        : null;

    if (containerRef.current && resizeObserver) {
      resizeObserver.observe(containerRef.current);
    }

    const timer = window.setTimeout(handleResize, 100);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      resizeObserver?.disconnect();
    };
  }, [displayedMetrics, monitorBarMode, snapshot]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      return;
    }

    const currentWindow = getCurrentWindow();

    currentWindow.setTitle("CoworkPal Monitor").catch((err) => {
      console.error("Failed to update monitor-bar taskbar title:", err);
    });
    currentWindow.setSkipTaskbar(true).catch((err) => {
      console.error("Failed to update monitor-bar taskbar visibility:", err);
    });
  }, []);

  return (
    <div className="cwp-transparent-root">
      <div
        ref={containerRef}
        aria-label="CoworkPal monitor bar"
        className={`cwp-monitor-bar cwp-glass-panel ${monitorModeClass(
          monitorBarMode,
        )}`}
        onDoubleClick={() => void openMainWindow()}
        onMouseDown={(event) => {
          if (event.button === 0) {
            dragCandidateRef.current = true;
            void startDraggingCurrentWindow();
          }
        }}
        onMouseUp={() => {
          window.setTimeout(() => {
            dragCandidateRef.current = false;
          }, 0);
        }}
        role="button"
        style={
          {
            "--monitor-count": Math.max(1, metrics.length),
            "--monitor-display-count": Math.max(1, displayedMetrics.length),
          } as React.CSSProperties
        }
        tabIndex={0}
      >
        <PetAvatar />
        <div className="cwp-monitor-content">
          {displayedMetrics.length > 0 ? (
            displayedMetrics.map((metric) => {
              const view = getMonitorMetricView(metric, snapshot);

              return (
                <MonitorChip
                  detail={view.detail}
                  key={metric}
                  label={view.label}
                  tone={view.tone}
                  value={view.value}
                />
              );
            })
          ) : (
            <span className="cwp-monitor-empty">未选择监控指标</span>
          )}
        </div>
      </div>
    </div>
  );
}

function monitorModeClass(mode: MonitorBarMode) {
  if (mode === "Micro") {
    return "is-micro";
  }

  if (mode === "Expanded") {
    return "is-expanded";
  }

  return "is-default";
}
