import { create } from "zustand";
import type { CatState, CatStateChangedEvent } from "../types/pet";
import { rewardLabel, type RewardAmount, type RewardNotice } from "../types/rewards";

export interface PetStore {
  catState: CatState;
  catMessage: string;
  isPanelOpen: boolean;
  setCatState: (state: CatState) => void;
  setCatMessage: (message: string) => void;
  setPetStatus: (event: CatStateChangedEvent) => void;
  rewardMessageUntil: number;
  recentRewardIds: string[];
  recentReward: RewardAmount | null;
  showReward: (notice: RewardNotice, bubbleEnabled: boolean) => void;
  openPanel: () => void;
  closePanel: () => void;
  togglePanel: () => void;
}

export const usePetStore = create<PetStore>((set) => ({
  catState: "Idle",
  catMessage: "CoCat 正在待命。",
  isPanelOpen: false,
  setCatState: (catState) => set({ catState }),
  setCatMessage: (catMessage) => set({ catMessage }),
  setPetStatus: (event) =>
    set((state) => ({
      catState: event.catState,
      catMessage: Date.now() < state.rewardMessageUntil
        && !["Hidden", "TemperatureCheck", "ErrorGlitch"].includes(event.catState)
        ? state.catMessage : event.catMessage,
    })),
  rewardMessageUntil: 0,
  recentRewardIds: [],
  recentReward: null,
  showReward: (notice, bubbleEnabled) => set((state) => {
    if (!bubbleEnabled || ["Hidden", "TemperatureCheck", "ErrorGlitch"].includes(state.catState)
      || !(notice.reward.parts || notice.reward.insight || notice.reward.affinityExperience)) {
      return state;
    }
    const inBurst = Date.now() < state.rewardMessageUntil;
    if (inBurst && state.recentRewardIds.includes(notice.rewardId)) return state;
    const previous = inBurst ? state.recentReward : null;
    const reward = {
      parts: (previous?.parts ?? 0) + notice.reward.parts,
      insight: (previous?.insight ?? 0) + notice.reward.insight,
      affinityExperience: (previous?.affinityExperience ?? 0) + notice.reward.affinityExperience,
    };
    return {
      catMessage: `${previous ? "奖励" : notice.title}已到账：${rewardLabel(reward)}`,
      rewardMessageUntil: Date.now() + 6000,
      recentRewardIds: [...(inBurst ? state.recentRewardIds : []), notice.rewardId],
      recentReward: reward,
    };
  }),
  openPanel: () => set({ isPanelOpen: true }),
  closePanel: () => set({ isPanelOpen: false }),
  togglePanel: () => set((state) => ({ isPanelOpen: !state.isPanelOpen })),
}));
