import type { WorkshopDecoration } from "../types/settings";
import { PixelIcon } from "../ui/PixelIcon";
import { useSettingsStore } from "../stores/settingsStore";
import { useWorkshopStore } from "../stores/workshopStore";
import { useFocusStore } from "../stores/focusStore";
import { decorationProgress } from "../services/workshopDecorations";

export function WorkshopDecorationVisual({ kind, compact = false }: { kind: Exclude<WorkshopDecoration, "none">; compact?: boolean }) {
  const title = kind === "orders" ? "工单纪念牌" : kind === "focus" ? "专注灯" : "整周纪念星";
  return <div role="img" aria-label={title} className={`cwp-decoration-art is-${kind}${compact ? " is-compact" : ""}`}>
    {kind === "focus" ? <><span className="cwp-lamp-light" /><span className="cwp-lamp-shade" /><span className="cwp-lamp-pole" /><span className="cwp-lamp-base" /></> : <>
      <div className="cwp-decoration-emblem"><PixelIcon name={kind === "orders" ? "tools" : "sparkle"} size={kind === "orders" ? 24 : 32} /></div>
      <span className="cwp-decoration-plinth" />
    </>}
  </div>;
}

export function CompanionDecoration() {
  const kind = useSettingsStore((state) => state.settings?.workshopDecoration ?? "none");
  const completedOrderCount = useWorkshopStore((state) => state.state?.completedOrderCount ?? 0);
  const rewardReceipts = useWorkshopStore((state) => state.state?.rewardReceipts);
  const sessions = useFocusStore((state) => state.book?.sessions);
  const progress = decorationProgress({ completedOrderCount, rewardReceipts: rewardReceipts ?? {} }, sessions ?? []);
  if (kind === "none" || progress[kind] < (kind === "weekly" ? 1 : 3)) return null;
  return <div className="cwp-companion-decoration" title="已陈列的成长纪念物"><WorkshopDecorationVisual kind={kind} compact /></div>;
}
