import { useEffect, useRef, useState } from "react";
import { getHardwareSnapshot } from "../../services/tauriCommands";
import { formatBytesPerSecond, formatPercent, formatTemperature } from "../../services/formatters";
import { useHardwareStore } from "../../stores/hardwareStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { MonitorMetric, PrimaryMetric } from "../../types/settings";
import { useThemedIcons } from "../../ui/assets";
import { PixelIcon, type PixelIconName } from "../../ui/PixelIcon";

const ONBOARDING_VERSION = 1;

const metricOptions: Array<{
  key: PrimaryMetric;
  label: string;
  icon: PixelIconName;
  detail: string;
}> = [
  { key: "Cpu", label: "CPU", icon: "cpu", detail: "计算负载" },
  { key: "Memory", label: "内存", icon: "ram", detail: "容量压力" },
  { key: "Temperature", label: "温度", icon: "temp", detail: "散热状态" },
  { key: "Network", label: "网络", icon: "network", detail: "传输速率" },
];

const metricPresets: Record<PrimaryMetric, MonitorMetric[]> = {
  Cpu: ["Cpu", "Gpu", "Ram", "Network"],
  Memory: ["Ram", "Cpu", "Disk", "Gpu"],
  Temperature: ["Cpu", "Gpu", "Ram"],
  Network: ["Network", "Cpu", "Ram", "Disk"],
};

export function OnboardingFlow() {
  const settings = useSettingsStore((state) => state.settings);
  const updateSettings = useSettingsStore((state) => state.updateSettings);
  const snapshot = useHardwareStore((state) => state.snapshot);
  const setSnapshot = useHardwareStore((state) => state.setSnapshot);
  const icons = useThemedIcons();
  const panelRef = useRef<HTMLDivElement>(null);
  const [step, setStep] = useState(0);
  const [catName, setCatName] = useState(settings?.catName || "CoCat");
  const [primaryMetric, setPrimaryMetric] = useState<PrimaryMetric>(
    settings?.primaryMetric || "Cpu",
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getHardwareSnapshot()
      .then(setSnapshot)
      .catch((loadError) => console.error("Failed to load onboarding hardware snapshot", loadError));
  }, [setSnapshot]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !busy) {
        event.preventDefault();
        void finish(true);
        return;
      }
      if (event.key !== "Tab" || !panelRef.current) {
        return;
      }
      const focusable = Array.from(
        panelRef.current.querySelectorAll<HTMLElement>(
          "button:not(:disabled), input:not(:disabled), [tabindex]:not([tabindex='-1'])",
        ),
      );
      if (focusable.length === 0) {
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  });

  async function finish(skipped = false) {
    setBusy(true);
    setError(null);
    try {
      if (skipped) {
        await updateSettings({ onboardingVersion: ONBOARDING_VERSION });
        return;
      }
      const name = catName.trim();
      if (!name || Array.from(name).length > 12) {
        setError("名称需要 1 到 12 个字符");
        setStep(0);
        return;
      }
      const metrics = metricPresets[primaryMetric];
      await updateSettings({
        catName: name,
        onboardingVersion: ONBOARDING_VERSION,
        primaryMetric,
        visibleMonitorMetrics: metrics,
        visibleTaskbarMetrics: metrics,
      });
    } catch (saveError) {
      setError(typeof saveError === "string" ? saveError : "保存失败，请重试");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="cwp-onboarding-backdrop">
      <div
        aria-describedby="cwp-onboarding-subtitle"
        aria-labelledby="cwp-onboarding-title"
        aria-modal="true"
        className="cwp-onboarding-panel"
        ref={panelRef}
        role="dialog"
      >
        <header className="cwp-onboarding-header">
          <div>
            <span id="cwp-onboarding-subtitle">初始校准 · {step + 1}/3</span>
            <h2 id="cwp-onboarding-title">{stepTitle(step)}</h2>
          </div>
          <button
            className="cwp-onboarding-skip"
            disabled={busy}
            onClick={() => void finish(true)}
            type="button"
          >
            跳过
          </button>
        </header>

        <div className="cwp-onboarding-progress" aria-hidden="true">
          {[0, 1, 2].map((index) => (
            <span className={index <= step ? "is-active" : ""} key={index} />
          ))}
        </div>

        <div className="cwp-onboarding-body">
          {step === 0 ? (
            <div className="cwp-onboarding-name-step">
              <img alt="CoCat" src={icons.cocatAvatar} />
              <label htmlFor="cwp-cat-name">这位工坊搭档怎么称呼？</label>
              <input
                autoFocus
                id="cwp-cat-name"
                maxLength={12}
                onChange={(event) => setCatName(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && catName.trim()) {
                    setStep(1);
                  }
                }}
                value={catName}
              />
              <span>{Array.from(catName.trim()).length}/12</span>
            </div>
          ) : null}

          {step === 1 ? (
            <fieldset className="cwp-onboarding-metrics">
              <legend>优先盯住哪项状态？</legend>
              <div>
                {metricOptions.map((option) => (
                  <button
                    aria-pressed={primaryMetric === option.key}
                    className={primaryMetric === option.key ? "is-active" : ""}
                    key={option.key}
                    onClick={() => setPrimaryMetric(option.key)}
                    type="button"
                  >
                    <PixelIcon name={option.icon} size={18} />
                    <strong>{option.label}</strong>
                    <span>{option.detail}</span>
                  </button>
                ))}
              </div>
            </fieldset>
          ) : null}

          {step === 2 ? (
            <div className="cwp-onboarding-live">
              <div className="cwp-onboarding-live-pet">
                <img alt={catName.trim() || "CoCat"} src={icons.cocatAvatar} />
                <strong>{catName.trim() || "CoCat"}</strong>
                <span>{liveMessage(primaryMetric, snapshot)}</span>
              </div>
              <div className="cwp-onboarding-live-reading">
                <span>{metricOptions.find((option) => option.key === primaryMetric)?.label}</span>
                <strong>{liveValue(primaryMetric, snapshot)}</strong>
                <small>{snapshot ? "实时采样" : "等待采样"}</small>
              </div>
            </div>
          ) : null}

          {error ? <div className="cwp-onboarding-error">{error}</div> : null}
        </div>

        <footer className="cwp-onboarding-actions">
          <button
            disabled={busy || step === 0}
            onClick={() => setStep((current) => Math.max(0, current - 1))}
            type="button"
          >
            上一步
          </button>
          {step < 2 ? (
            <button
              className="is-primary"
              disabled={busy || (step === 0 && !catName.trim())}
              onClick={() => setStep((current) => Math.min(2, current + 1))}
              type="button"
            >
              下一步
            </button>
          ) : (
            <button
              className="is-primary"
              disabled={busy}
              onClick={() => void finish()}
              type="button"
            >
              {busy ? "保存中..." : "开始陪伴"}
            </button>
          )}
        </footer>
      </div>
    </div>
  );
}

function stepTitle(step: number) {
  return ["认识你的 CoCat", "选择关注重点", "查看此刻状态"][step] || "初始校准";
}

type Snapshot = ReturnType<typeof useHardwareStore.getState>["snapshot"];

function liveValue(metric: PrimaryMetric, snapshot: Snapshot) {
  if (!snapshot) {
    return "--";
  }
  switch (metric) {
    case "Cpu":
      return formatPercent(snapshot.cpuUsagePercent);
    case "Memory":
      return formatPercent(snapshot.memoryUsagePercent);
    case "Temperature":
      return formatTemperature(snapshot.cpuTemperatureCelsius);
    case "Network":
      return formatBytesPerSecond(
        (snapshot.networkDownloadBytesPerSecond ?? 0) +
          (snapshot.networkUploadBytesPerSecond ?? 0),
      );
  }
}

function liveMessage(metric: PrimaryMetric, snapshot: Snapshot) {
  if (!snapshot) {
    return "正在等待第一份硬件采样。";
  }
  if (metric === "Cpu" && (snapshot.cpuUsagePercent ?? 0) >= 80) {
    return "计算任务正忙，我已经进入抢修节奏。";
  }
  if (metric === "Memory" && (snapshot.memoryUsagePercent ?? 0) >= 82) {
    return "内存空间偏紧，我正在整理零件仓库。";
  }
  if (metric === "Temperature" && (snapshot.cpuTemperatureCelsius ?? 0) >= 75) {
    return "温度升高，我会继续盯住冷却状态。";
  }
  if (
    metric === "Network" &&
    (snapshot.networkDownloadBytesPerSecond ?? 0) +
      (snapshot.networkUploadBytesPerSecond ?? 0) >=
      10 * 1024 * 1024
  ) {
    return "传输站很忙，数据正在高速通过。";
  }
  return "状态稳定，我会在桌面陪你一起工作。";
}
