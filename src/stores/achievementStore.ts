import { create } from "zustand";
import {
  getAchievementSummary,
  getWeeklyGoals,
  listAchievements,
  markAchievementNotificationsSeen,
} from "../services/tauriCommands";
import type {
  AchievementBucketSummary,
  AchievementCard,
  AchievementSummary,
  AchievementUnlockedEvent,
  WeeklyGoals,
} from "../types/achievement";

export interface AchievementStore {
  summary: AchievementSummary | null;
  cards: AchievementCard[];
  unlockQueue: AchievementUnlockedEvent[];
  weeklyGoals: WeeklyGoals | null;
  loadSummary: () => Promise<AchievementSummary>;
  loadCards: () => Promise<void>;
  loadWeeklyGoals: () => Promise<void>;
  pushUnlocked: (event: AchievementUnlockedEvent) => void;
  markNotificationsSeen: (unlockIds?: string[]) => Promise<AchievementSummary>;
  dismissUnlocked: (unlockId: string) => void;
}

function createUnlockedCard(
  event: AchievementUnlockedEvent,
  existing?: AchievementCard,
): AchievementCard {
  return {
    achievementId: event.achievementId,
    title: event.title,
    categoryKey: event.categoryKey,
    difficultyKey: event.difficultyKey,
    points: event.points,
    badgeKey: event.badgeKey,
    isHidden: event.isHidden,
    isUnlocked: true,
    unlockedAt: event.unlockedAt,
    unlockSnapshot: event.unlockSnapshot ?? null,
    conditionSummary: existing?.conditionSummary ?? null,
    progress: existing?.progress
      ? { ...existing.progress, percent: 100, isComplete: true }
      : {
          current: 1,
          target: 1,
          percent: 100,
          label: "已解锁",
          isComplete: true,
        },
  };
}

function incrementBucket<T extends string>(
  buckets: Record<T, AchievementBucketSummary>,
  key: T,
): Record<T, AchievementBucketSummary> {
  const bucket = buckets[key];
  if (!bucket) {
    return buckets;
  }

  return {
    ...buckets,
    [key]: {
      ...bucket,
      unlocked: bucket.unlocked + 1,
    },
  } as Record<T, AchievementBucketSummary>;
}

export const useAchievementStore = create<AchievementStore>((set) => ({
  summary: null,
  cards: [],
  unlockQueue: [],
  weeklyGoals: null,
  loadSummary: async () => {
    const summary = await getAchievementSummary();
    set({ summary });
    return summary;
  },
  loadCards: async () => {
    const cards = await listAchievements(true);
    set({ cards });
  },
  loadWeeklyGoals: async () => {
    const weeklyGoals = await getWeeklyGoals();
    set({ weeklyGoals });
  },
  pushUnlocked: (event) =>
    set((state) => {
      const isDuplicateEvent = state.unlockQueue.some(
        (item) => item.unlockId === event.unlockId,
      );
      if (isDuplicateEvent) {
        return state;
      }

      const existingCard = state.cards.find(
        (card) => card.achievementId === event.achievementId,
      );
      const wasAlreadyUnlocked = existingCard?.isUnlocked ?? false;
      const unlockedCard = createUnlockedCard(event, existingCard);
      const cards =
        state.cards.length > 0
          ? existingCard
            ? state.cards.map((card) =>
                card.achievementId === event.achievementId
                  ? unlockedCard
                  : card,
              )
            : [...state.cards, unlockedCard]
          : state.cards;
      const summary =
        state.summary && !wasAlreadyUnlocked
          ? {
              ...state.summary,
              totalPoints: state.summary.totalPoints + event.points,
              unlockedCount: state.summary.unlockedCount + 1,
              hiddenUnlockedCount:
                state.summary.hiddenUnlockedCount + (event.isHidden ? 1 : 0),
              pendingNotificationCount:
                state.summary.pendingNotificationCount + 1,
              byDifficulty: incrementBucket(
                state.summary.byDifficulty,
                event.difficultyKey,
              ),
              byCategory: incrementBucket(
                state.summary.byCategory,
                event.categoryKey,
              ),
              latestUnlocks: [
                unlockedCard,
                ...state.summary.latestUnlocks.filter(
                  (card) => card.achievementId !== event.achievementId,
                ),
              ].slice(0, 3),
            }
          : state.summary;

      return {
        cards,
        summary,
        weeklyGoals: state.weeklyGoals,
        unlockQueue: [...state.unlockQueue, event],
      };
    }),
  markNotificationsSeen: async (unlockIds) => {
    const summary = await markAchievementNotificationsSeen(unlockIds);
    set({ summary });
    return summary;
  },
  dismissUnlocked: (unlockId) =>
    set((state) => ({
      unlockQueue: state.unlockQueue.filter(
        (event) => event.unlockId !== unlockId,
      ),
    })),
}));
