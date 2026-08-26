import { useEffect, useRef, useState } from "react";
import {
  hideMonitorBar,
  hidePetWindow,
  showMonitorBar,
  showPetWindow,
  exitApp,
  downloadUserData,
  checkAccessTokenRequest,
  getMemoryStatus,
  getSyncConfig,
  requestAccessToken,
  triggerMemoryRelease,
  updateSyncConfig,
  uploadUserData,
  type MemoryStatus,
} from "../../services/tauriCommands";
import { usePetStore } from "../../stores/petStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import type { AppSettingsPatch, MonitorBarMode, MonitorMetric } from "../../types/settings";
import type { SyncConfig } from "../../types/cloudSync";
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

const cloudBackupContentLabels = [
  "应用设置",
  "设备配置",
  "工坊进度",
  "日报",
  "体检",
  "成就",
  "专注记录",
  "笔记",
  "窗口布局",
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
  const applyOptimistic = useSettingsStore((state) => state.applyOptimistic);
  // Fire-and-forget wrapper for discrete settings writes (toggles, theme, ...).
  // updateSettings rethrows on failure (after reconciling state via re-fetch),
  // so we swallow the rejection here to avoid unhandled-promise warnings — the
  // store has already corrected the UI, there is nothing else to surface.
  const safeUpdate = (patch: AppSettingsPatch) => {
    void updateSettings(patch).catch((error) => {
      console.error("failed to persist settings", error);
    });
  };
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
  const [syncConfig, setSyncConfig] = useState<SyncConfig>({
    serverUrl: "",
    accessToken: "",
    userName: "",
    tokenRequestId: "",
    tokenRequestSecret: "",
    autoBackupEnabled: false,
    autoBackupIntervalMinutes: 30,
  });
  const [syncBusy, setSyncBusy] = useState<
    "save" | "request" | "check" | "upload" | "download" | null
  >(null);
  const [syncMessage, setSyncMessage] = useState("");
  const tokenCheckInFlightRef = useRef(false);

  // Slider/continuous inputs are coalesced here: every drag fires many onChange
  // events, and we must not invoke updateAppSettings (a disk write + IPC round
  // trip) per pixel. The latest pending patch is held in a ref and flushed once
  // the user pauses for DEBOUNCE_MS. The store applies each patch optimistically
  // immediately, so the on-screen value stays live; only persistence is gated.
  const settingsDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Pending patches are MERGED, not replaced: dragging slider A then slider B
  // within the debounce window must persist both, not drop A's change.
  const pendingSettingsPatch = useRef<AppSettingsPatch>({});
  function scheduleSettingsCommit(patch: AppSettingsPatch) {
    // Apply the value to local state immediately so the slider/thumb and its
    // numeric label stay perfectly live during a drag.
    applyOptimistic(patch);
    // Merge into the pending patch so all sliders touched this window persist.
    Object.assign(pendingSettingsPatch.current, patch);
    if (settingsDebounceRef.current) {
      clearTimeout(settingsDebounceRef.current);
    }
    settingsDebounceRef.current = setTimeout(() => {
      settingsDebounceRef.current = null;
      const patchToCommit = pendingSettingsPatch.current;
      pendingSettingsPatch.current = {};
      // updateSettings rethrows on failure (after re-fetching authoritative
      // state). Catch here to avoid an unhandled rejection — the store has
      // already reconciled, so there is nothing else for the UI to do.
      void updateSettings(patchToCommit).catch((error) => {
        console.error("failed to persist settings", error);
      });
    }, 250);
  }
  useEffect(() => {
    return () => {
      if (settingsDebounceRef.current) {
        // Flush the pending patch instead of discarding it, so a quick
        // drag-then-leave doesn't lose the last value (it was already shown
        // optimistically; this persists it).
        clearTimeout(settingsDebounceRef.current);
        const pending = pendingSettingsPatch.current;
        pendingSettingsPatch.current = {};
        if (Object.keys(pending).length > 0) {
          void updateSettings(pending).catch((error) =>
            console.error("failed to flush settings on unmount", error),
          );
        }
      }
    };
  }, []);

  useEffect(() => {
    let active = true;
    void getSyncConfig()
      .then((config) => {
        if (active) setSyncConfig(config);
      })
      .catch((error) => {
        if (active) setSyncMessage(`读取同步配置失败：${String(error)}`);
      });
    return () => {
      active = false;
    };
  }, []);

  async function saveCloudConfig() {
    const saved = await updateSyncConfig(syncConfig);
    setSyncConfig(saved);
    return saved;
  }

  async function saveSyncSchedule(
    patch: Pick<SyncConfig, "autoBackupEnabled" | "autoBackupIntervalMinutes">,
  ) {
    if (syncBusy) return;
    const previous = syncConfig;
    const next = { ...syncConfig, ...patch };
    setSyncConfig(next);
    setSyncBusy("save");
    setSyncMessage("");
    try {
      setSyncConfig(await updateSyncConfig(next));
      setSyncMessage("自动备份设置已保存");
    } catch (error) {
      setSyncConfig(await getSyncConfig().catch(() => previous));
      setSyncMessage(`保存自动备份设置失败：${String(error)}`);
    } finally {
      setSyncBusy(null);
    }
  }

  async function handleTokenRequest() {
    if (syncBusy) return;
    setSyncBusy("request");
    setSyncMessage("");
    try {
      const result = await requestAccessToken(syncConfig);
      setSyncConfig(await getSyncConfig());
      setSyncMessage(
        `已提交 ${result.userName} 的令牌申请，管理员发放后会自动保存令牌并上传首次备份`,
      );
    } catch (error) {
      setSyncMessage(`申请失败：${String(error)}`);
    } finally {
      setSyncBusy(null);
    }
  }

  async function checkTokenRequest(manual: boolean) {
    if (tokenCheckInFlightRef.current) return;
    tokenCheckInFlightRef.current = true;
    if (manual) setSyncBusy("check");
    try {
      const result = await checkAccessTokenRequest();
      setSyncConfig(await getSyncConfig());
      if (result.status === "approved") {
        if (result.initialSync) {
          const time = new Date(result.initialSync.storedAt).toLocaleString();
          setSyncMessage(`令牌已自动保存，并已上传首次完整备份 · ${time}`);
        } else {
          setSyncMessage(
            `令牌已自动保存，但首次备份失败：${result.initialSyncError ?? "未知错误"}`,
          );
        }
      } else if (result.status === "rejected") {
        setSyncMessage("管理员未通过本次令牌申请，请确认用户名后重新申请");
      } else if (manual) {
        setSyncMessage("申请仍在等待管理员处理");
      }
    } catch (error) {
      if (manual) {
        setSyncMessage(`检查申请失败：${String(error)}`);
      } else {
        const latest = await getSyncConfig().catch(() => null);
        if (latest) {
          setSyncConfig(latest);
          if (latest.accessToken) {
            setSyncMessage("令牌已自动保存，首次完整备份已由客户端处理");
          }
        }
      }
    } finally {
      tokenCheckInFlightRef.current = false;
      if (manual) setSyncBusy(null);
    }
  }

  useEffect(() => {
    if (!syncConfig.tokenRequestId || syncConfig.accessToken) return;
    void checkTokenRequest(false);
    const timer = setInterval(() => void checkTokenRequest(false), 10_000);
    return () => clearInterval(timer);
  }, [syncConfig.tokenRequestId, syncConfig.accessToken]);

  async function handleCloudAction(action: "save" | "upload" | "download") {
    if (syncBusy) return;
    if (
      action === "download" &&
      !window.confirm("从云端恢复会覆盖本机的设置、工坊、笔记及其他用户数据。确认继续吗？")
    ) {
      return;
    }

    setSyncBusy(action);
    setSyncMessage("");
    try {
      await saveCloudConfig();
      if (action === "save") {
        setSyncMessage("同步配置已保存");
        return;
      }
      const result = action === "upload" ? await uploadUserData() : await downloadUserData();
      const time = new Date(result.storedAt).toLocaleString();
      setSyncMessage(
        action === "upload"
          ? `已上传完整备份 · ${time}`
          : `已从云端恢复 · ${time} · 请重启应用以完整应用窗口与启动配置`,
      );
    } catch (error) {
      setSyncMessage(`操作失败：${String(error)}`);
    } finally {
      setSyncBusy(null);
    }
  }

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
      safeUpdate({ isCatVisible: checked });
      return;
    }
    void (checked ? showPetWindow() : hidePetWindow());
  }

  function setMonitorBarVisible(checked: boolean) {
    if (!isTauriRuntime) {
      safeUpdate({ isMonitorBarVisible: checked });
      return;
    }
    void (checked ? showMonitorBar() : hideMonitorBar());
  }

  async function handleToggleCoCatPause() {
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
        ? "CoCat 已暂停工坊生产，进入沉睡。"
        : "CoCat 已唤醒，工坊生产继续运行。",
    });
  }

  const confirmExit = async () => {
    if (window.confirm("确认退出 CoworkPal 吗?")) {
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
              <img src={icons.cocatAvatar} alt="CoCat Avatar Large" />
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
              <div className="cwp-portrait-name">CoCat / 工程猫</div>
              <div className="cwp-portrait-desc">您的个人硬件工坊诊断助理</div>
            </div>
          </div>
          <div className="cwp-portrait-actions">
            <button
              className="cwp-portrait-btn primary"
              onClick={() => void handleToggleCoCatPause().catch((error) => console.error("toggle production pause failed", error))}
              type="button"
            >
              {settings?.isProductionPaused ? "唤醒 CoCat" : "暂停 / 沉睡"}
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
                  onChange={(e) => scheduleSettingsCommit({ catSize: Number(e.target.value) })}
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
                  onChange={(e) => scheduleSettingsCommit({ catOpacity: Number(e.target.value) })}
                />
                <span className="slider-val">
                  {Math.round((settings?.catOpacity ?? 0.95) * 100)}%
                </span>
              </div>

              <div className="cwp-settings-row-inline" style={{ marginTop: "2px" }}>
                <span className="cwp-settings-label">宠物皮肤</span>
                <select
                  className="cwp-custom-select"
                  value={settings?.themeName || "coworkpal"}
                  onChange={(e) => safeUpdate({ themeName: e.target.value })}
                  style={{ height: "20px", padding: "0 4px" }}
                >
                  <option value="coworkpal">默认皮肤</option>
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
                      onChange={(e) => safeUpdate({ enablePetBubble: e.target.checked })}
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
                      onChange={(e) => safeUpdate({ enableStaticCatMode: e.target.checked })}
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
                        onClick={() => safeUpdate({ monitorBarMode: mode.key })}
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
                          safeUpdate({ visibleMonitorMetrics: nextMetrics });
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

            {/* Card: Taskbar settings */}
            <div className="cwp-settings-card cwp-settings-card-taskbar">
              <div className="cwp-settings-card-title">
                <PixelIcon name="puzzle" size={14} style={{ marginRight: "6px" }} /> 任务栏设置
              </div>
              <div className="cwp-settings-switches-grid" style={{ gridTemplateColumns: "repeat(2, 1fr)", gap: "4px" }}>
                <div className="cwp-switch-item-inline">
                  <span className="cwp-settings-label">显示任务栏</span>
                  <label className="cwp-switch-label">
                    <input
                      type="checkbox"
                      checked={settings?.showMonitorDataInTaskbar ?? false}
                      onChange={(e) =>
                        safeUpdate({
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
                        onClick={() => safeUpdate({ taskbarMonitorMode: mode.key })}
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
                          safeUpdate({ visibleTaskbarMetrics: nextMetrics });
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
                      onChange={(e) => safeUpdate({ enableLowPowerMode: e.target.checked })}
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
                      onChange={(e) => safeUpdate({ launchAtStartup: e.target.checked })}
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
                      onChange={(e) => safeUpdate({ enableNotifications: e.target.checked })}
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
                  onChange={(e) => safeUpdate({ enableSound: e.target.value !== "none" })}
                  style={{ height: "20px", padding: "0 4px" }}
                >
                  <option value="none">静音</option>
                  <option value="enabled">开启音效</option>
                </select>
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
                      scheduleSettingsCommit({
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
                      scheduleSettingsCommit({
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
                      scheduleSettingsCommit({
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
                      scheduleSettingsCommit({
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
                      scheduleSettingsCommit({
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
                      safeUpdate({ memoryReleaseEnabled: e.target.checked })
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
                      safeUpdate({
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
                    scheduleSettingsCommit({
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

            {/* Card: bundled hardware sensor helper */}
            <div className="cwp-settings-card">
              <div className="cwp-settings-card-title">
                <PixelIcon name="monitor" size={14} style={{ marginRight: "6px" }} /> 硬件监控
              </div>
              <div className="cwp-settings-row-inline">
                <span className="cwp-settings-label">内置硬件温度监控</span>
                <label className="cwp-switch-label">
                  <input
                    type="checkbox"
                    aria-label="内置硬件温度监控"
                    checked={settings?.integratedHardwareMonitorEnabled ?? false}
                    onChange={(e) =>
                      safeUpdate({
                        integratedHardwareMonitorEnabled: e.target.checked,
                      })
                    }
                  />
                  <span className="cwp-switch-slider"></span>
                </label>
              </div>
              <div className="cwp-settings-note">
                直接读取 CPU / GPU 硬件传感器。首次开启会请求管理员权限，并自动安装随应用提供的硬件访问驱动。
              </div>
            </div>

            <div className="cwp-settings-card">
              <div className="cwp-settings-card-title">
                <PixelIcon name="devices" size={14} style={{ marginRight: "6px" }} /> 云端数据备份
              </div>
              <label className="cwp-sync-field">
                <span>服务器地址</span>
                <input
                  aria-label="同步服务器地址"
                  autoComplete="url"
                  onChange={(event) =>
                    setSyncConfig((current) => ({ ...current, serverUrl: event.target.value }))
                  }
                  placeholder="http://192.168.124.13:18080"
                  type="url"
                  value={syncConfig.serverUrl}
                />
              </label>
              <label className="cwp-sync-field">
                <span>用户名</span>
                <input
                  aria-label="同步用户名"
                  autoComplete="username"
                  disabled={Boolean(syncConfig.tokenRequestId)}
                  maxLength={40}
                  onChange={(event) =>
                    setSyncConfig((current) => ({ ...current, userName: event.target.value }))
                  }
                  placeholder="用于管理员识别，例如 CoCatFan"
                  type="text"
                  value={syncConfig.userName}
                />
              </label>
              <label className="cwp-sync-field">
                <span>访问令牌</span>
                <input
                  aria-label="同步访问令牌"
                  autoComplete="off"
                  onChange={(event) =>
                    setSyncConfig((current) => ({ ...current, accessToken: event.target.value }))
                  }
                  placeholder="服务器分配的访问令牌"
                  type="password"
                  value={syncConfig.accessToken}
                />
              </label>
              <div className="cwp-sync-schedule">
                <div className="cwp-settings-row-inline">
                  <span className="cwp-settings-label">自动备份上传</span>
                  <label className="cwp-switch-label">
                    <input
                      aria-label="自动备份上传"
                      checked={syncConfig.autoBackupEnabled}
                      disabled={syncBusy !== null || !syncConfig.accessToken}
                      onChange={(event) =>
                        void saveSyncSchedule({
                          autoBackupEnabled: event.target.checked,
                          autoBackupIntervalMinutes: syncConfig.autoBackupIntervalMinutes,
                        })
                      }
                      type="checkbox"
                    />
                    <span className="cwp-switch-slider" />
                  </label>
                </div>
                <label className="cwp-sync-interval">
                  <span>上传间隔</span>
                  <select
                    aria-label="自动备份上传间隔"
                    className="cwp-custom-select"
                    disabled={
                      syncBusy !== null ||
                      !syncConfig.autoBackupEnabled ||
                      !syncConfig.accessToken
                    }
                    onChange={(event) =>
                      void saveSyncSchedule({
                        autoBackupEnabled: syncConfig.autoBackupEnabled,
                        autoBackupIntervalMinutes: Number(event.target.value),
                      })
                    }
                    value={syncConfig.autoBackupIntervalMinutes}
                  >
                    <option value={15}>15 分钟</option>
                    <option value={30}>30 分钟</option>
                    <option value={60}>1 小时</option>
                    <option value={120}>2 小时</option>
                    <option value={360}>6 小时</option>
                  </select>
                </label>
              </div>
              <div className="cwp-sync-contents">
                <span className="cwp-sync-contents-title">备份内容</span>
                <div className="cwp-sync-contents-grid">
                  {cloudBackupContentLabels.map((label) => (
                    <span key={label} className="cwp-sync-content-item">
                      <span aria-hidden="true" />
                      {label}
                    </span>
                  ))}
                </div>
              </div>
              {syncConfig.tokenRequestId && !syncConfig.accessToken ? (
                <div className="cwp-sync-request-status" role="status">
                  <span className="cwp-sync-request-dot" aria-hidden="true" />
                  等待管理员发放令牌，客户端每 10 秒自动检查
                </div>
              ) : null}
              <div className="cwp-sync-actions">
                <button
                  className="cwp-metric-select"
                  disabled={syncBusy !== null}
                  onClick={() => void handleCloudAction("save")}
                  type="button"
                >
                  保存配置
                </button>
                {!syncConfig.accessToken ? (
                  <button
                    className="cwp-metric-select is-active"
                    disabled={syncBusy !== null}
                    onClick={() =>
                      void (syncConfig.tokenRequestId
                        ? checkTokenRequest(true)
                        : handleTokenRequest())
                    }
                    type="button"
                  >
                    {syncBusy === "request"
                      ? "申请中…"
                      : syncBusy === "check"
                        ? "检查中…"
                        : syncConfig.tokenRequestId
                          ? "检查申请状态"
                          : "申请令牌"}
                  </button>
                ) : null}
                <button
                  className="cwp-metric-select is-active"
                  disabled={syncBusy !== null || !syncConfig.accessToken}
                  onClick={() => void handleCloudAction("upload")}
                  type="button"
                >
                  {syncBusy === "upload" ? "上传中…" : "上传完整备份"}
                </button>
                <button
                  className="cwp-metric-select"
                  disabled={syncBusy !== null || !syncConfig.accessToken}
                  onClick={() => void handleCloudAction("download")}
                  type="button"
                >
                  {syncBusy === "download" ? "恢复中…" : "从云端恢复"}
                </button>
              </div>
              {syncMessage ? <div className="cwp-sync-message">{syncMessage}</div> : null}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
