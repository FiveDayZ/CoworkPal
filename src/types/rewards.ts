import type { FocusSession } from "./focus";

export interface RewardAmount {
  parts: number;
  insight: number;
  affinityExperience: number;
}

export interface RewardReceipt {
  title: string;
  reward: RewardAmount;
  earnedAt: number;
  paidAt: number;
}

export interface RewardNotice {
  rewardId: string;
  title: string;
  reward: RewardAmount;
}

export function rewardLabel(reward: RewardAmount): string {
  const number = (value: number) =>
    new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 1 }).format(value);
  return `零件 +${number(reward.parts)} · 灵感 +${number(reward.insight)}${
    reward.affinityExperience ? ` · 亲密经验 +${reward.affinityExperience}` : ""
  }`;
}

export function creditedSeconds(session: FocusSession, now?: number): number {
  if (!session.rewardVersion) {
    return Math.min(session.plannedDurationSeconds,
      Math.max(0, ((session.endedAt ?? now ?? session.startedAt) - session.startedAt) / 1000));
  }
  const delta = session.status === "active" && now != null && session.lastTickAt != null
    ? now - session.lastTickAt : 0;
  return Math.floor(Math.min(session.plannedDurationSeconds,
    (session.creditedDurationMs + (delta >= 0 && delta <= 30_000 ? delta : 0)) / 1000));
}

export function estimateFocusReward(seconds: number, distractions = 0): RewardAmount {
  const quality = Math.max(0.4, 1 - distractions * 0.15);
  return { parts: seconds >= 300 ? seconds / 60 * 8 * quality : 0,
    insight: seconds >= 300 ? seconds / 60 * 0.5 * quality : 0, affinityExperience: 0 };
}
