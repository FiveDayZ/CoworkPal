import { useState } from "react";
import { useSettingsStore } from "../stores/settingsStore";
import type { CompanionMode } from "../types/settings";
const MODES: Array<[CompanionMode, string]> = [["normal", "日常陪伴"], ["quiet", "安静陪伴"], ["economy", "节能陪伴"]];
export function CompanionModeControl() {
  const settings = useSettingsStore((state) => state.settings);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function select(companionMode: CompanionMode) {
    setBusy(true); setError(null);
    try { await useSettingsStore.getState().updateSettings({ companionMode }); }
    catch (error) { setError(String(error)); }
    finally { setBusy(false); }
  }
  return <section className="cwp-companion-modes" aria-label="陪伴场景">
    <div className="cwp-inline-actions">{MODES.map(([mode, label]) => <button type="button" key={mode} className="cwp-compact-button" disabled={!settings || busy} aria-pressed={(settings?.companionMode ?? "normal") === mode} onClick={() => void select(mode)}>{label}</button>)}</div>
    <p>{settings?.companionMode === "economy" ? "静态陪伴并降低采样频率，专注计时照常运行。" : settings?.companionMode === "quiet" ? "暂时关闭声音、通知和气泡，桌宠保持静态。" : "按你的声音、动画和通知偏好陪伴。"} 切回日常可恢复原偏好。</p>
    {error ? <p role="alert">场景切换失败：{error}</p> : null}
  </section>;
}
