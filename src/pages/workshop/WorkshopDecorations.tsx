import { useState } from "react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import { useFocusStore } from "../../stores/focusStore";
import { decorationProgress } from "../../services/workshopDecorations";
import type { WorkshopDecoration } from "../../types/settings";
import { PixelIcon, type PixelIconName } from "../../ui/PixelIcon";
import { useThemedIcons } from "../../ui/assets";
import { WorkshopDecorationVisual } from "../../components/WorkshopDecorationVisual";
const DECORATIONS: Array<{ id: Exclude<WorkshopDecoration, "none">; title: string; icon: PixelIconName; target: number; hint: string }> = [
  { id: "orders", title: "工单纪念牌", icon: "tools", target: 3, hint: "完成 3 张工单" },
  { id: "focus", title: "专注灯", icon: "focus", target: 3, hint: "完成 3 次至少 5 分钟的专注" },
  { id: "weekly", title: "整周纪念星", icon: "sparkle", target: 1, hint: "领取一次整周目标奖励" },
];
export function WorkshopDecorations() {
  const workshop = useWorkshopStore((state) => state.state);
  const sessions = useFocusStore((state) => state.book?.sessions);
  const settings = useSettingsStore((state) => state.settings);
  const icons = useThemedIcons();
  const [preview, setPreview] = useState<Exclude<WorkshopDecoration, "none">>(settings?.workshopDecoration && settings.workshopDecoration !== "none" ? settings.workshopDecoration : "orders");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const progress = decorationProgress(workshop, sessions ?? []);
  const item = DECORATIONS.find((item) => item.id === preview)!;
  const unlocked = progress[item.id] >= item.target;
  const equipped = settings?.workshopDecoration === item.id && unlocked;
  async function select(workshopDecoration: WorkshopDecoration) {
    setBusy(true); setError(null);
    try { await useSettingsStore.getState().updateSettings({ workshopDecoration }); }
    catch (error) { setError(String(error)); }
    finally { setBusy(false); }
  }
  return <section className="cwp-decoration-shelf" aria-label="工坊陈列台">
    <div className="cwp-section-title"><strong>{item.title}</strong><span>{equipped ? "已陈列 · 在控制台与 CoCat 一起展示" : unlocked ? "已解锁，可以陈列" : "预览 · 完成目标后解锁"}</span></div>
    <div className={`cwp-decoration-stage is-${item.id}${unlocked ? " is-unlocked" : " is-preview"}`}>
      <div className="cwp-decoration-stage-wall"><i /><i /><i /></div>
      <img src={icons.cocatAvatar} className="cwp-decoration-cat" alt="CoCat 与成长纪念物" />
      <div className="cwp-decoration-stage-item"><WorkshopDecorationVisual kind={item.id} /></div>
      <div className="cwp-decoration-bench" />
      <div className="cwp-decoration-stage-actions">
        <span>{unlocked ? "成长纪念物" : `${item.hint} · ${progress[item.id]}/${item.target}`}</span>
        <button type="button" className="cwp-compact-button is-primary" disabled={busy || !settings || !unlocked || equipped} onClick={() => void select(item.id)}>{busy ? "保存中…" : equipped ? "已陈列" : unlocked ? "陈列到工坊" : "尚未解锁"}</button>
        {settings?.workshopDecoration && settings.workshopDecoration !== "none" ? <button type="button" className="cwp-compact-button" disabled={busy} onClick={() => void select("none")}>收起装饰</button> : null}
      </div>
    </div>
    <div className="cwp-decoration-options" aria-label="纪念物预览">{DECORATIONS.map((entry) => <button type="button" key={entry.id} className="cwp-compact-button" disabled={busy} aria-pressed={preview === entry.id} onClick={() => { setPreview(entry.id); setError(null); }}>
      <PixelIcon name={entry.icon} size={18} /><strong>{entry.title}{settings?.workshopDecoration === entry.id && progress[entry.id] >= entry.target ? " · 已陈列" : ""}</strong>
      <span>{entry.hint}</span><progress aria-label={`${entry.title}解锁进度`} max={entry.target} value={progress[entry.id]} /><span>{progress[entry.id]}/{entry.target} · {progress[entry.id] >= entry.target ? "已解锁" : "可预览"}</span>
    </button>)}</div>
    {error ? <p className="cwp-decoration-error" role="alert">装饰未保存：{error}</p> : null}
  </section>;
}
