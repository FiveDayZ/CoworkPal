import type { FocusSession } from "../types/focus";
import type { WorkshopState } from "../types/workshop";
import { creditedSeconds } from "../types/rewards";
export function decorationProgress(workshop: Pick<WorkshopState, "completedOrderCount" | "rewardReceipts"> | null, sessions: FocusSession[]) {
  return {
    none: 1,
    orders: Math.min(3, workshop?.completedOrderCount ?? 0),
    focus: Math.min(3, sessions.filter((session) => session.status === "completed" && creditedSeconds(session) >= 300).length),
    weekly: Object.keys(workshop?.rewardReceipts ?? {}).some((id) => id.startsWith("week:") && id.endsWith(":all")) ? 1 : 0,
  };
}
