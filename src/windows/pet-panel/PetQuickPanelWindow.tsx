import type React from "react";
import { useEffect, useRef, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import { PixelIcon } from "../../ui/PixelIcon";
import {
  formatBytesPerSecond,
  formatParts,
  formatPercent,
  formatTemperature,
} from "../../services/formatters";
import {
  exitApp,
  hidePetPanel,
  hidePetWindow,
  showMainWindow,
  toggleMonitorBar,
  saveWindowPosition,
} from "../../services/tauriCommands";
import { calculateSystemStability } from "../../services/systemStability";
import { startDraggingCurrentWindow } from "../../services/windowApi";
import { useHardwareStore } from "../../stores/hardwareStore";
import { useFocusStore } from "../../stores/focusStore";
import { usePetStore } from "../../stores/petStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import {
  CompactButton,
  GlassPanel,
  MiniSlider,
  PetAvatar,
  StatusBadge,
  ToggleSwitch,
} from "../../ui/components";
import { percentLevel } from "../../ui/metrics";
import { RollingValue } from "../../ui/RollingValue";

const panelRecycleDurationMs = 220;

function notifyPanelClose() {
  if (!("__TAURI_INTERNALS__" in window)) {
    return;
  }

  void emit("corecat:interaction-state", "panelClose");
}

export function PetQuickPanelWindow() {
  const snapshot = useHardwareStore((state) => state.snapshot);
  const workshop = useWorkshopStore((state) => state.state);
  const catState = usePetStore((state) => state.catState);
  const catMessage = usePetStore((state) => state.catMessage);
  const settings = useSettingsStore((state) => state.settings);
  const updateSettings = useSettingsStore((state) => state.updateSettings);
  const loadSettings = useSettingsStore((state) => state.loadSettings);
  const activeFocusSession = useFocusStore((state) => state.activeSession);
  const isStartingFocus = useFocusStore((state) => state.isStarting);
  const startFocus = useFocusStore((state) => state.start);
  const completeFocus = useFocusStore((state) => state.complete);
  const abandonFocus = useFocusStore((state) => state.abandon);

  const [menuOpen, setMenuOpen] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const [focusTask, setFocusTask] = useState("");
  const [focusMinutes, setFocusMinutes] = useState(25);
  const [focusError, setFocusError] = useState<string | null>(null);
  const [remainingSeconds, setRemainingSeconds] = useState(0);
  const closeTimerRef = useRef<number | null>(null);

  const dragCandidateRef = useRef(false);
  const dragMovedRef = useRef(false);
  const dragStartRef = useRef({ x: 0, y: 0 });

  useEffect(() => {
    function handlePointerUp() {
      dragCandidateRef.current = false;
      dragMovedRef.current = false;
    }
    window.addEventListener("pointerup", handlePointerUp);
    return () => {
      window.removeEventListener("pointerup", handlePointerUp);
    };
  }, []);

  useEffect(
    () => () => {
      if (closeTimerRef.current != null) {
        window.clearTimeout(closeTimerRef.current);
      }
    },
    [],
  );

  // Live countdown for an active focus session.
  useEffect(() => {
    if (!activeFocusSession) {
      setRemainingSeconds(0);
      return undefined;
    }
    function tick() {
      if (!activeFocusSession) {
        return;
      }
      const elapsed = Math.floor((Date.now() - activeFocusSession.startedAt) / 1000);
      setRemainingSeconds(Math.max(0, activeFocusSession.plannedDurationSeconds - elapsed));
    }
    tick();
    const timer = window.setInterval(tick, 1000);
    return () => window.clearInterval(timer);
  }, [activeFocusSession]);

  async function handleStartFocus() {
    setFocusError(null);
    try {
      await startFocus(focusTask.trim(), focusMinutes);
      setFocusTask("");
    } catch (error) {
      setFocusError(typeof error === "string" ? error : "无法开始专注会话");
    }
  }

  async function toggleMonitor() {
    await toggleMonitorBar();
    await loadSettings();
    setMenuOpen(false);
  }

  async function confirmExit() {
    if (window.confirm("确认退出 CoreWorkPal 吗?")) {
      await exitApp();
    }
  }

  function closePanelWithRecycle() {
    if (isClosing) {
      return;
    }

    setMenuOpen(false);
    setIsClosing(true);
    notifyPanelClose();
    closeTimerRef.current = window.setTimeout(() => {
      closeTimerRef.current = null;
      void hidePetPanel().finally(() => setIsClosing(false));
    }, panelRecycleDurationMs);
  }

  async function resetPositions() {
    await saveWindowPosition("pet", 1200, 520);
    await saveWindowPosition("monitor-bar", 580, 24);
    await loadSettings();
    setMenuOpen(false);
    alert("位置已重置。");
  }

  // Calculate dynamic multipliers from current hardware usage
  const cpu = snapshot?.cpuUsagePercent ?? 0;
  const memory = snapshot?.memoryUsagePercent ?? 0;
  const activity = (cpu * 0.7 + memory * 0.3) / 100;
  const efficiency = Math.min(1.8, Math.max(0.8, 0.8 + activity));
  const stability = calculateSystemStability(snapshot, settings);

  return (
    <div
      className={`cwp-transparent-root cwp-pet-panel-root ${
        isClosing ? "is-closing" : ""
      }`}
    >
      {/* Arrow pointing right towards the pet companion */}
      <div className="cwp-panel-arrow cwp-panel-arrow-right" />

      <GlassPanel
        className="cwp-pet-panel"
        style={{ position: "relative" }}
        onMouseDown={(event: React.MouseEvent<HTMLElement>) => {
          const target = event.target as HTMLElement;
          if (
            target.closest("button") ||
            target.closest(".cwp-secondary-menu") ||
            target.closest(".cwp-toggle-switch") ||
            target.closest(".cwp-mini-slider") ||
            target.closest("input")
          ) {
            return;
          }
          if (event.button === 0) {
            dragCandidateRef.current = true;
            dragMovedRef.current = false;
            dragStartRef.current = { x: event.screenX, y: event.screenY };
          }
        }}
        onMouseMove={(event: React.MouseEvent<HTMLElement>) => {
          if (!dragCandidateRef.current || event.buttons !== 1) {
            return;
          }
          const deltaX = Math.abs(event.screenX - dragStartRef.current.x);
          const deltaY = Math.abs(event.screenY - dragStartRef.current.y);
          if (!dragMovedRef.current && (deltaX > 3 || deltaY > 3)) {
            dragMovedRef.current = true;
            void startDraggingCurrentWindow();
          }
        }}
      >
        <header className="cwp-panel-header">
          <div className="cwp-panel-title">
            <PetAvatar />
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <strong>CoreCat</strong>
              <span className="cwp-panel-status-dot" />
            </div>
          </div>
          <div className="cwp-panel-actions">
            <button
              aria-label="Settings Menu"
              className="panel-icon-btn"
              onClick={() => setMenuOpen(!menuOpen)}
              type="button"
              style={{ background: "transparent", cursor: "pointer", display: "inline-flex", alignItems: "center", justifyContent: "center", border: "none", color: "var(--color-text-primary)" }}
            >
              <PixelIcon name="settings" size={14} />
            </button>
            <button
              aria-label="Close Panel"
              className="panel-icon-btn"
              onClick={closePanelWithRecycle}
              type="button"
              style={{ background: "transparent", cursor: "pointer", display: "inline-flex", alignItems: "center", justifyContent: "center", border: "none", color: "var(--color-text-primary)" }}
            >
              <PixelIcon name="close" size={12} />
            </button>
          </div>
        </header>

        {/* Secondary cog settings popup menu */}
        <div className={`cwp-secondary-menu ${menuOpen ? "show" : ""}`}>
          <div className="cwp-secondary-menu-item" onClick={() => { void showMainWindow(); setMenuOpen(false); }} style={{ display: "flex", alignItems: "center" }}>
            <PixelIcon name="settings" size={12} style={{ marginRight: "6px" }} /> 打开主工坊
          </div>
          <div className="cwp-secondary-menu-item" onClick={() => void resetPositions()}>
            ⟲ 重置位置
          </div>
          <div className="cwp-secondary-menu-item" onClick={() => void toggleMonitor()} style={{ display: "flex", alignItems: "center" }}>
            <PixelIcon name="monitor" size={12} style={{ marginRight: "6px" }} /> {settings?.isMonitorBarVisible ? "隐藏监控条" : "显示监控条"}
          </div>
          <div className="cwp-secondary-menu-item danger" onClick={() => void confirmExit()} style={{ display: "flex", alignItems: "center" }}>
            <PixelIcon name="close" size={12} style={{ marginRight: "6px" }} /> 退出程序
          </div>
        </div>

        <p className="cwp-panel-message" style={{ minHeight: "34px", margin: "10px 0" }}>
          {catMessage || "CoreCat 正在观察当前工作区。"}
        </p>

        <div className="cwp-panel-chips">
          <PanelChip
            label="CPU"
            level={percentLevel(snapshot?.cpuUsagePercent)}
            tone="orange"
            value={formatPercent(snapshot?.cpuUsagePercent ?? null)}
          />
          <PanelChip
            label="RAM"
            level={percentLevel(snapshot?.memoryUsagePercent)}
            tone="cyan"
            value={formatPercent(snapshot?.memoryUsagePercent ?? null)}
          />
          <PanelChip
            label="TEMP"
            level={percentLevel(snapshot?.cpuTemperatureCelsius)}
            tone="red"
            value={formatTemperature(snapshot?.cpuTemperatureCelsius ?? null)}
          />
        </div>

        {/* Extended state tags */}
        <div className="cwp-panel-status-row">
          <div className="cwp-panel-status-item">
            效率: <span>{`高效 (x${efficiency.toFixed(1)})`}</span>
          </div>
          <div className="cwp-panel-status-item">
            稳定: <span>{stability.label}</span>
          </div>
        </div>

        {/* Focus ritual: deliver a task to CoreCat and stay heads-down. */}
        <div className="cwp-panel-focus">
          {activeFocusSession ? (
            <div className="cwp-panel-focus-active">
              <div className="cwp-panel-focus-task" title={activeFocusSession.taskLabel}>
                <PixelIcon name="focus" size={14} className="cwp-panel-focus-task-icon" />
                <span>{activeFocusSession.taskLabel}</span>
              </div>
              <div className="cwp-panel-focus-timer">
                {String(Math.floor(remainingSeconds / 60)).padStart(2, "0")}:
                {String(remainingSeconds % 60).padStart(2, "0")}
              </div>
              <div className="cwp-panel-focus-meta">
                分心 {activeFocusSession.distractionCount} 次
              </div>
              <div className="cwp-panel-focus-actions">
                <CompactButton onClick={() => void completeFocus()} variant="primary">
                  完成
                </CompactButton>
                <CompactButton onClick={() => void abandonFocus()}>放弃</CompactButton>
              </div>
            </div>
          ) : (
            <div className="cwp-panel-focus-setup">
              <input
                className="cwp-panel-focus-input"
                type="text"
                placeholder="向 CoreCat 交付一个任务…"
                value={focusTask}
                maxLength={40}
                onChange={(event) => setFocusTask(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && focusTask.trim() && !isStartingFocus) {
                    void handleStartFocus();
                  }
                }}
              />
              <div className="cwp-panel-focus-options">
                {[25, 50].map((minutes) => (
                  <button
                    key={minutes}
                    type="button"
                    className={
                      "cwp-panel-focus-chip" +
                      (focusMinutes === minutes ? " is-active" : "")
                    }
                    onClick={() => setFocusMinutes(minutes)}
                  >
                    {minutes} 分钟
                  </button>
                ))}
              </div>
              {focusError ? (
                <div className="cwp-panel-focus-error">{focusError}</div>
              ) : null}
              <CompactButton
                onClick={() => void handleStartFocus()}
                variant="primary"
                disabled={!focusTask.trim() || isStartingFocus}
              >
                {isStartingFocus ? "开始中…" : "开始专注"}
              </CompactButton>
            </div>
          )}
        </div>

        <div className="cwp-panel-button-grid">
          <CompactButton onClick={() => void showMainWindow()} variant="primary">
            打开
          </CompactButton>
          <CompactButton
            onClick={() =>
              void updateSettings({
                isProductionPaused: !(settings?.isProductionPaused ?? false),
              })
            }
          >
            {settings?.isProductionPaused ? "继续" : "暂停"}
          </CompactButton>
          <CompactButton onClick={() => void toggleMonitor()}>
            监控
          </CompactButton>
          <CompactButton onClick={() => void hidePetWindow()}>
            隐藏
          </CompactButton>
        </div>

        <MiniSlider
          label="大小"
          max={1.5}
          min={0.6}
          onChange={(value) => void updateSettings({ catSize: value })}
          step={0.05}
          value={settings?.catSize ?? 1}
          valueLabel={`${Math.round((settings?.catSize ?? 1) * 100)}%`}
        />
        <MiniSlider
          label="透明"
          max={1}
          min={0.2}
          onChange={(value) => void updateSettings({ catOpacity: value })}
          step={0.05}
          value={settings?.catOpacity ?? 0.95}
          valueLabel={`${Math.round((settings?.catOpacity ?? 0.95) * 100)}%`}
        />

        <div className="cwp-panel-toggles" style={{ marginTop: "10px" }}>
          <ToggleSwitch
            checked={settings?.enablePetBubble ?? true}
            label="气泡提示"
            onChange={(checked) => void updateSettings({ enablePetBubble: checked })}
          />
          <ToggleSwitch
            checked={settings?.enableStaticCatMode ?? false}
            label="静态模式"
            onChange={(checked) =>
              void updateSettings({ enableStaticCatMode: checked })
            }
          />
          <ToggleSwitch
            checked={settings?.enableLowPowerMode ?? false}
            label="低功耗"
            onChange={(checked) =>
              void updateSettings({ enableLowPowerMode: checked })
            }
          />
        </div>
      </GlassPanel>
    </div>
  );
}

function PanelChip({
  label,
  level,
  tone,
  value,
}: {
  label: string;
  level: number;
  tone: "cyan" | "orange" | "red";
  value: string;
}) {
  return (
    <div
      className={`cwp-panel-chip tone-${tone}`}
      style={{ "--metric-level": `${level}%` } as React.CSSProperties}
    >
      <span>{label}</span>
      <strong>
        <RollingValue value={value} />
      </strong>
    </div>
  );
}
