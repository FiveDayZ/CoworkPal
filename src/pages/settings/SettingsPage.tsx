import { useEffect, useState } from "react";
import {
  hideMonitorBar,
  hidePetWindow,
  showMonitorBar,
  showPetWindow,
  exitApp,
  getMemoryStatus,
  triggerMemoryRelease,
  type MemoryStatus,
} from "../../services/tauriCommands";
import { usePetStore } from "../../stores/petStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import type { MonitorBarMode, MonitorMetric } from "../../types/settings";
import { defaultModuleLevels } from "../../types/workshop";
import { useThemedIcons } from "../../ui/assets";
import { PixelIcon } from "../../ui/PixelIcon";

const monitorMetricOptions: Array<{ key: MonitorMetric; label: string }> = [
  { key: "Cpu", label: "CPU" },
  { key: "Ram", label: "RAM" },
  { key: "Disk", label: "DISK" },
  { key: "Network", label: "NET" },
  { key: "Gpu", label: "GPU" },
];

const monitorModeOptions: Array<{ key: MonitorBarMode; label: string }> = [
  { key: "Micro", label: "Micro" },
  { key: "Default", label: "Default" },
  { key: "Expanded", label: "Expanded" },
];

/** Compact `MM-DD HH:mm` format for the "last release" line so the whole value
 *  fits on a single row inside the narrow settings card. */
function formatLastReleaseTime(timestampMs: number): string {
  const date = new Date(timestampMs);
  const mm = String(date.getMonth() + 1).padStart(2, "0");
  const dd = String(date.getDate()).padStart(2, "0");
  const hh = String(date.getHours()).padStart(2, "0");
  const min = String(date.getMinutes()).padStart(2, "0");
  return `${mm}-${dd} ${hh}:${min}`;
}

export function SettingsPage() {
  const settings = useSettingsStore((state) => state.settings);
  const icons = useThemedIcons();
  const updateSettings = useSettingsStore((state) => state.updateSettings);
  const setPetStatus = usePetStore((state) => state.setPetStatus);
  const saveWorkshopState = useWorkshopStore((store) => store.saveWorkshopState);
  const visibleMetrics = settings?.visibleMonitorMetrics ?? [];
  const visibleTaskbarMetrics =
    settings?.visibleTaskbarMetrics ?? settings?.visibleMonitorMetrics ?? [];
  const isTauriRuntime =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  // Live system memory snapshot for the memory-release card. Refreshed every
  // 5s while the settings page is mounted. `null` until the first sample.
  const [memoryStatus, setMemoryStatus] = useState<MemoryStatus | null>(null);
  // True while a manual release is in flight (UAC prompt + helper run). Keeps
  // the button from being double-clicked and gives affordance feedback.
  const [releasing, setReleasing] = useState(false);

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setInterval> | undefined;
    async function refresh() {
      try {
        const status = await getMemoryStatus();
        if (active) {
          setMemoryStatus(status);
        }
      } catch (error) {
        console.warn("failed to read memory status", error);
      }
    }
    void refresh();
    timer = setInterval(() => void refresh(), 5000);
    return () => {
      active = false;
      if (timer) {
        clearInterval(timer);
      }
    };
  }, []);

  async function handleReleaseMemory() {
    if (releasing) {
      return;
    }
    setReleasing(true);
    try {
      await triggerMemoryRelease();
      // Refresh the live reading so the "current usage" line updates after
      // the release; the "last release" line is driven by settings:updated.
      try {
        const status = await getMemoryStatus();
        setMemoryStatus(status);
      } catch {
        // best-effort — the toast still fires from the event listener
      }
    } catch (error) {
      console.error("memory release failed", error);
      if (!isTauriRuntime) {
        alert("内存释放失败，请稍后重试。");
      }
    } finally {
      setReleasing(false);
    }
  }

  function setCatVisible(checked: boolean) {
    if (!isTauriRuntime) {
      void updateSettings({ isCatVisible: checked });
      return;
    }
    void (checked ? showPetWindow() : hidePetWindow());
  }

  function setMonitorBarVisible(checked: boolean) {
    if (!isTauriRuntime) {
      void updateSettings({ isMonitorBarVisible: checked });
      return;
    }
    void (checked ? showMonitorBar() : hideMonitorBar());
  }

  async function handleToggleCoreCatPause() {
    const isPaused = settings?.isProductionPaused ?? false;
    const nextPaused = !isPaused;

    await updateSettings({ isProductionPaused: nextPaused });

    if (!nextPaused && isTauriRuntime) {
      await showPetWindow();
    }

    setPetStatus({
      timestamp: Date.now(),
      catState: nextPaused ? "Sleep" : "Idle",
      catMessage: nextPaused
        ? "CoreCat 已暂停工坊生产，进入沉睡。"
        : "CoreCat 已唤醒，工坊生产继续运行。",
    });
  }

  const confirmExit = async () => {
    if (window.confirm("确认退出 CoreWorkPal 吗?")) {
      await exitApp();
    }
  };

  const handleResetAll = async () => {
    if (window.confirm("确认要重置所有工坊状态和已获得的零件/灵感数据吗？该操作无法撤销。")) {
      await saveWorkshopState({
        schemaVersion: 1,
        parts: 280.0,
        insight: 12.0,
        workshopLevel: 1,
        catAffinityLevel: 1,
        moduleLevels: defaultModuleLevels,
        lastProductionTime: Date.now(),
        totalOnlineSeconds: 0,
        todayParts: 0.0,
        todayInsight: 0.0,
        lastDailyResetDate: new Date().toISOString().split("T")[0],
      });
      alert("工坊状态已成功重置为初始状态！");
    }
  };

  return (
    <div className="cwp-page">
      <div className="page-title-row">
        <h2 className="page-title">工坊配置控制台</h2>
      </div>

      <div className="cwp-settings-grid">
        {/* Left Column: Chibi Companion Portrait Card */}
        <div className="cwp-settings-portrait-card">
          <div className="cwp-settings-portrait-top-section">
            <div className="cwp-portrait-avatar-wrapper">
              <img src={icons.corecatAvatar} alt="CoreCat Avatar Large" />
            </div>
            {settings?.catId && (
              <div
                className="cwp-portrait-cat-id"
                title="您的宠物唯一识别码 CatID"
              >
                ID: {settings.catId}
              </div>
            )}
            <div className="cwp-portrait-info">
              <div className="cwp-portrait-name">CoreCat / 工程猫</div>
              <div className="cwp-portrait-desc">您的个人硬件工坊诊断助理</div>
            </div>
          </div>
          <div className="cwp-portrait-actions">
            <button
              className="cwp-portrait-btn primary"
              onClick={() => void handleToggleCoreCatPause()}
              type="button"
            >
              {settings?.isProductionPaused ? "唤醒 CoreCat" : "暂停 / 沉睡"}
            </button>
            <button
              className="cwp-portrait-btn danger"
              onClick={() => void confirmExit()}
              type="button"
            >
              退出工坊系统
            </button>
            <button
              className="cwp-portrait-btn danger"
              onClick={() => void handleResetAll()}
              type="button"
              style={{ marginTop: "4px" }}
            >
              重置工坊数据
            </button>
          </div>
        </div>

        {/* Right Column: categories cards */}
        <div className="cwp-settings-cards-scroll">
          {/* Column 1: Cat settings and Monitor bar settings */}
          <div className="cwp-settings-column">
            {/* Card 1: Companion Settings */}
            <div className="cwp-settings-card cwp-settings-card-desktop">
              <div className="cwp-settings-card-title">
                <PixelIcon name="cat" size={14} style={{ marginRight: "6px" }} /> 桌宠设置
              </div>
              <div className="cwp-settings-row-inline">
                <span className="cwp-settings-label">宠物大小</span>
                <input
                  type="range"
                  min="0.6"
                  max="1.5"
                  step="0.05"
                  value={settings?.catSize ?? 1}
                  className="custom-range"
                  onChange={(e) => void updateSettings({ catSize: Number(e.target.value) })}
                />
                <span className="slider-val">
                  {Math.round((settings?.catSize ?? 1) * 100)}%
                </span>
              </div>
              <div className="cwp-settings-row-inline">
                <span className="cwp-settings-label">宠物透明度</span>
                <input
                  type="range"
                  min="0.2"
                  max="1"
                  step="0.05"
                  value={settings?.catOpacity ?? 0.95}
                  className="custom-range"
                  onChange={(e) => void updateSettings({ catOpacity: Number(e.target.value) })}
                />
                <span className="slider-val">
                  {Math.round((settings?.catOpacity ?? 0.95) * 100)}%
                </span>
              </div>

              <div className="cwp-settings-row-inline" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label">宠物皮肤</span>
                <select
                  className="cwp-custom-select"
                  value={settings?.themeName || "coreworkpal"}
                  onChange={(e) => void updateSettings({ themeName: e.target.value })}
                  style={{ height: "20px", padding: "0 4px" }}
                >
                  <option value="coreworkpal">默认皮肤</option>
                  <option value="classic">暖心布丁橙</option>
                  <option value="cyber">梦幻苏打蓝</option>
                  <option value="steampunk">甜心蜜桃粉</option>
                </select>
              </div>

              <div className="cwp-settings-switches-grid" style={{ gridTemplateColumns: "repeat(3, 1fr)", gap: "2px", marginTop: "2px" }}>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">显示</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.isCatVisible ?? true}
                      onChange={(e) => setCatVisible(e.target.checked)}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">气泡</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.enablePetBubble ?? true}
                      onChange={(e) => void updateSettings({ enablePetBubble: e.target.checked })}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">静态</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.enableStaticCatMode ?? false}
                      onChange={(e) => void updateSettings({ enableStaticCatMode: e.target.checked })}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
              </div>
            </div>

            {/* Card 2: MonitorBar settings */}
            <div className="cwp-settings-card cwp-settings-card-monitor">
              <div className="cwp-settings-card-title">
                <PixelIcon name="monitor" size={14} style={{ marginRight: "6px" }} /> 监控栏设置
              </div>
              <div className="cwp-settings-switches-grid" style={{ gridTemplateColumns: "repeat(2, 1fr)", gap: "4px" }}>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">显示监控条</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.isMonitorBarVisible ?? false}
                      onChange={(e) => setMonitorBarVisible(e.target.checked)}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
              </div>

              <div className="cwp-settings-row" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label" style={{ fontSize: "11px" }}>监控条模式</span>
                <div className="cwp-settings-metric-grid is-compact" style={{ gap: "4px", marginTop: "2px" }}>
                  {monitorModeOptions.map((mode) => {
                    const active = (settings?.monitorBarMode ?? "Default") === mode.key;
                    return (
                      <button
                        className={`cwp-metric-select ${active ? "is-active" : ""}`}
                        key={mode.key}
                        onClick={() => void updateSettings({ monitorBarMode: mode.key })}
                        type="button"
                      >
                        {mode.label}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="cwp-settings-row" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label" style={{ fontSize: "11px" }}>显示指标</span>
                <div className="cwp-settings-metric-grid" style={{ gridTemplateColumns: "repeat(5, minmax(0, 1fr))", gap: "2px", marginTop: "2px" }}>
                  {monitorMetricOptions.map((metric) => {
                    const checked = visibleMetrics.includes(metric.key);
                    return (
                      <button
                        className={`cwp-metric-select ${checked ? "is-active" : ""}`}
                        key={metric.key}
                        onClick={() => {
                          const nextMetrics = checked
                            ? visibleMetrics.filter((item) => item !== metric.key)
                            : [...visibleMetrics, metric.key];
                          void updateSettings({ visibleMonitorMetrics: nextMetrics });
                        }}
                        style={{ fontSize: "9px" }}
                        type="button"
                      >
                        {metric.label}
                      </button>
                    );
                  })}
                </div>
              </div>

            </div>

            {/* Card 3: Running setup */}
            <div className="cwp-settings-card cwp-settings-card-runtime">
              <div className="cwp-settings-card-title">
                <PixelIcon name="energy" size={14} style={{ marginRight: "6px" }} /> 运行与特效
              </div>
              <div className="cwp-settings-switches-grid" style={{ gridTemplateColumns: "repeat(3, 1fr)", gap: "2px" }}>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">低功耗</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.enableLowPowerMode ?? false}
                      onChange={(e) => void updateSettings({ enableLowPowerMode: e.target.checked })}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>

                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">自启动</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.launchAtStartup ?? false}
                      onChange={(e) => void updateSettings({ launchAtStartup: e.target.checked })}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>

                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">通知</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.enableNotifications ?? false}
                      onChange={(e) => void updateSettings({ enableNotifications: e.target.checked })}
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
              </div>

              <div className="cwp-settings-row-inline" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label">声音反馈</span>
                <select
                  className="cwp-custom-select"
                  value={settings?.enableSound ? "enabled" : "none"}
                  onChange={(e) => void updateSettings({ enableSound: e.target.value !== "none" })}
                  style={{ height: "20px", padding: "0 4px" }}
                >
                  <option value="none">静音</option>
                  <option value="enabled">开启音效</option>
                </select>
              </div>
            </div>

            {/* Card 4: Security */}
            <div className="cwp-settings-card cwp-settings-card-security">
              <div className="cwp-settings-card-title">
                <PixelIcon name="shield" size={14} style={{ marginRight: "6px" }} /> 安全与隐私
              </div>
              <div className="cwp-safety-notes-box" style={{ padding: "4px 6px", gap: "2px" }}>
                <div className="cwp-safety-note-item">轻量绿色免安装</div>
                <div className="cwp-safety-note-item">本地离线不联网</div>
                <div className="cwp-safety-note-item">后台无静默占用</div>
                <div className="cwp-safety-note-item">用户数据不上传</div>
              </div>
            </div>
          </div>

          {/* Column 2: Run settings and Safety notes */}
          <div className="cwp-settings-column">
            {/* Card: Thresholds (action trigger values) */}
            <div className="cwp-settings-card cwp-settings-card-thresholds">
              <div className="cwp-settings-card-title">
                <PixelIcon name="slider" size={14} style={{ marginRight: "6px" }} /> 动作触发值
              </div>
              <div className="cwp-settings-threshold-grid">
                <div className="cwp-settings-row-inline">
                  <span className="cwp-settings-label">Working CPU</span>
                  <input
                    className="custom-range"
                    max="95"
                    min="10"
                    onChange={(e) =>
                      void updateSettings({
                        dataSortingCpuThreshold: Number(e.target.value),
                      })
                    }
                    step="1"
                    type="range"
                    value={settings?.dataSortingCpuThreshold ?? 40}
                  />
                  <span className="slider-val">
                    {Math.round(settings?.dataSortingCpuThreshold ?? 40)}%
                  </span>
                </div>
                <div className="cwp-settings-row-inline">
                  <span className="cwp-settings-label">Memory Crowded</span>
                  <input
                    className="custom-range"
                    max="98"
                    min="50"
                    onChange={(e) =>
                      void updateSettings({
                        memoryCrowdedThreshold: Number(e.target.value),
                      })
                    }
                    step="1"
                    type="range"
                    value={settings?.memoryCrowdedThreshold ?? 82}
                  />
                  <span className="slider-val">
                    {Math.round(settings?.memoryCrowdedThreshold ?? 82)}%
                  </span>
                </div>
                <div className="cwp-settings-row-inline">
                  <span className="cwp-settings-label">Temperature CPU</span>
                  <input
                    className="custom-range"
                    max="100"
                    min="75"
                    onChange={(e) =>
                      void updateSettings({
                        cpuTemperatureWarning: Number(e.target.value),
                      })
                    }
                    step="1"
                    type="range"
                    value={settings?.cpuTemperatureWarning ?? 80}
                  />
                  <span className="slider-val">
                    {Math.round(settings?.cpuTemperatureWarning ?? 80)}℃
                  </span>
                </div>
                <div className="cwp-settings-row-inline">
                  <span className="cwp-settings-label">Temperature GPU</span>
                  <input
                    className="custom-range"
                    max="105"
                    min="75"
                    onChange={(e) =>
                      void updateSettings({
                        gpuTemperatureWarning: Number(e.target.value),
                      })
                    }
                    step="1"
                    type="range"
                    value={settings?.gpuTemperatureWarning ?? 82}
                  />
                  <span className="slider-val">
                    {Math.round(settings?.gpuTemperatureWarning ?? 82)}℃
                  </span>
                </div>
                <div className="cwp-settings-row-inline">
                  <span className="cwp-settings-label">ErrorGlitch CPU</span>
                  <input
                    className="custom-range"
                    max="100"
                    min="50"
                    onChange={(e) =>
                      void updateSettings({
                        errorGlitchCpuThreshold: Number(e.target.value),
                      })
                    }
                    step="1"
                    type="range"
                    value={settings?.errorGlitchCpuThreshold ?? 96}
                  />
                  <span className="slider-val">
                    {Math.round(settings?.errorGlitchCpuThreshold ?? 96)}%
                  </span>
                </div>
              </div>
            </div>

            {/* Card: Taskbar data settings */}
            <div className="cwp-settings-card cwp-settings-card-taskbar">
              <div className="cwp-settings-card-title">
                <PixelIcon name="puzzle" size={14} style={{ marginRight: "6px" }} /> 任务栏数据
              </div>
              <div className="cwp-settings-switches-grid" style={{ gridTemplateColumns: "repeat(2, 1fr)", gap: "4px" }}>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">显示任务栏</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.showMonitorDataInTaskbar ?? false}
                      onChange={(e) =>
                        void updateSettings({
                          showMonitorDataInTaskbar: e.target.checked,
                        })
                      }
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
              </div>

              <div className="cwp-settings-row" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label" style={{ fontSize: "11px" }}>任务栏模式</span>
                <div className="cwp-settings-metric-grid is-compact" style={{ gap: "4px", marginTop: "2px" }}>
                  {monitorModeOptions.map((mode) => {
                    const active = (settings?.taskbarMonitorMode ?? "Default") === mode.key;
                    return (
                      <button
                        className={`cwp-metric-select ${active ? "is-active" : ""}`}
                        key={mode.key}
                        onClick={() => void updateSettings({ taskbarMonitorMode: mode.key })}
                        type="button"
                      >
                        {mode.label}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="cwp-settings-row" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label" style={{ fontSize: "11px" }}>显示指标</span>
                <div className="cwp-settings-metric-grid" style={{ gridTemplateColumns: "repeat(5, minmax(0, 1fr))", gap: "2px", marginTop: "2px" }}>
                  {monitorMetricOptions.map((metric) => {
                    const checked = visibleTaskbarMetrics.includes(metric.key);
                    return (
                      <button
                        className={`cwp-metric-select ${checked ? "is-active" : ""}`}
                        key={metric.key}
                        onClick={() => {
                          const nextMetrics = checked
                            ? visibleTaskbarMetrics.filter((item) => item !== metric.key)
                            : [...visibleTaskbarMetrics, metric.key];
                          void updateSettings({ visibleTaskbarMetrics: nextMetrics });
                        }}
                        style={{ fontSize: "9px" }}
                        type="button"
                      >
                        {metric.label}
                      </button>
                    );
                  })}
                </div>
              </div>

            </div>

            {/* Card: Memory Release */}
            <div className="cwp-settings-card cwp-settings-card-memory">
              <div className="cwp-settings-card-title">
                <PixelIcon name="ram" size={14} style={{ marginRight: "6px" }} /> 内存释放
              </div>
              <div className="cwp-settings-row-inline">
                <span className="cwp-settings-label">启用内存释放</span>
                <label className="cwp-switch-label">
                  <input
                    type="checkbox"
                    checked={settings?.memoryReleaseEnabled ?? true}
                    onChange={(e) =>
                      void updateSettings({ memoryReleaseEnabled: e.target.checked })
                    }
                  />
                  <span className="cwp-switch-slider"></span>
                </label>
              </div>
              <div className="cwp-settings-row-inline">
                <span className="cwp-settings-label">超过阈值自动释放</span>
                <label className="cwp-switch-label">
                  <input
                    type="checkbox"
                    checked={settings?.memoryAutoReleaseEnabled ?? false}
                    onChange={(e) =>
                      void updateSettings({
                        memoryAutoReleaseEnabled: e.target.checked,
                      })
                    }
                  />
                  <span className="cwp-switch-slider"></span>
                </label>
              </div>
              <div className="cwp-settings-row-inline">
                <span className="cwp-settings-label">自动释放阈值</span>
                <input
                  className="custom-range"
                  max="64"
                  min="1"
                  onChange={(e) =>
                    void updateSettings({
                      memoryAutoReleaseThresholdGib: Number(e.target.value),
                    })
                  }
                  step="0.5"
                  type="range"
                  value={settings?.memoryAutoReleaseThresholdGib ?? 8}
                />
                <span className="slider-val">
                  {(settings?.memoryAutoReleaseThresholdGib ?? 8).toFixed(1)} GB
                </span>
              </div>
              <div
                className="cwp-settings-row-inline"
                style={{ flexWrap: "nowrap", whiteSpace: "nowrap", gap: "4px 12px" }}
              >
                <span className="cwp-settings-label">当前系统占用</span>
                <span className="slider-val">
                  {memoryStatus
                    ? `${memoryStatus.usedGib.toFixed(1)} / ${(memoryStatus.totalBytes / 1024 / 1024 / 1024).toFixed(0)} GB (${memoryStatus.loadPercent}%)`
                    : "采样中…"}
                </span>
              </div>
              {settings?.memoryLastRelease ? (
                <div
                  className="cwp-settings-row-inline"
                  style={{ flexWrap: "nowrap", whiteSpace: "nowrap", gap: "4px 12px" }}
                >
                  <span className="cwp-settings-label">上次释放</span>
                  <span className="slider-val">
                    {formatLastReleaseTime(settings.memoryLastRelease.timestampMs)}
                    {` · ${(settings.memoryLastRelease.releasedBytes / 1024 / 1024 / 1024).toFixed(2)}GB`}
                    {settings.memoryLastRelease.fullTier ? " 全量" : " 轻量"}
                  </span>
                </div>
              ) : null}
              <div className="cwp-settings-row-inline" style={{ marginTop: "2px" }}>
                <button
                  type="button"
                  className="cwp-metric-select is-active"
                  disabled={
                    releasing || !(settings?.memoryReleaseEnabled ?? true)
                  }
                  onClick={() => void handleReleaseMemory()}
                  style={{ flex: 1, justifyContent: "center", cursor: releasing ? "wait" : "pointer" }}
                >
                  {releasing ? "释放中…" : "立即释放内存"}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
