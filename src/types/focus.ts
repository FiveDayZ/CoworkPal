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
}

export interface FocusSessionBook {
  schemaVersion: number;
  sessions: FocusSession[];
}
