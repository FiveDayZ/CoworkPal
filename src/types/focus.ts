import type { RewardAmount } from "./rewards";

export type FocusSessionStatus = "active" | "completed" | "abandoned";

export interface FocusSession {
  id: string;
  taskLabel: string;
  plannedDurationSeconds: number;
  startedAt: number;
  endedAt: number | null;
  status: FocusSessionStatus;
  distractionCount: number;
  focusQuality: number;
  productionMultiplier: number;
  rewardVersion: number;
  creditedDurationMs: number;
  lastTickAt: number | null;
  reward: RewardAmount | null;
  rewardPaid: boolean;
  achievementRecorded: boolean;
}

export interface FocusSessionBook {
  schemaVersion: number;
  sessions: FocusSession[];
}
