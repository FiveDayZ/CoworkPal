export type AchievementDifficulty =
  | "entry"
  | "normal"
  | "skilled"
  | "elite"
  | "epic"
  | "legendary";

export type AchievementCategory =
  | "daily_use"
  | "task_efficiency"
  | "long_streak"
  | "feature_exploration"
  | "data_milestone"
  | "workshop_growth"
  | "hardware_health"
  | "social_collaboration"
  | "hidden_easter";

export interface TrackAchievementEventRequest {
  eventName: string;
  occurredAt: number;
  idempotencyKey: string;
  payload: Record<string, unknown>;
  source?: string;
}

export interface AchievementUnlockedEvent {
  unlockId: string;
  achievementId: string;
  title: string;
  difficultyKey: AchievementDifficulty;
  categoryKey: AchievementCategory;
  points: number;
  badgeKey: string;
  unlockedAt: number;
  isHidden: boolean;
  unlockSnapshot: Record<string, number> | null;
  cocatAnimationState: "achievementPop";
}

export interface TrackAchievementEventResponse {
  accepted: boolean;
  unlocked: AchievementUnlockedEvent[];
}

export interface AchievementCard {
  achievementId: string;
  title: string;
  categoryKey: AchievementCategory;
  difficultyKey: AchievementDifficulty;
  points: number;
  badgeKey: string;
  isHidden: boolean;
  isUnlocked: boolean;
  unlockedAt: number | null;
  unlockSnapshot: Record<string, number> | null;
  conditionSummary: string | null;
  progress: AchievementProgress | null;
}

export interface AchievementProgress {
  current: number;
  target: number;
  percent: number;
  label: string;
  isComplete: boolean;
}

export interface AchievementBucketSummary {
  unlocked: number;
  total: number;
}

export interface AchievementSummary {
  totalPoints: number;
  unlockedCount: number;
  visibleTotalCount: number;
  hiddenUnlockedCount: number;
  hiddenTotalCount: number;
  byDifficulty: Record<AchievementDifficulty, AchievementBucketSummary>;
  byCategory: Record<AchievementCategory, AchievementBucketSummary>;
  latestUnlocks: AchievementCard[];
  highestRankUnlocks: AchievementCard[];
  pendingNotificationCount: number;
}
