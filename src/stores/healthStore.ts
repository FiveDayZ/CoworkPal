import { create } from "zustand";
import { getHealthTrend } from "../services/tauriCommands";
import type { HealthTrendReport, TrendRange } from "../types/health";

export interface HealthStore {
  trendReport: HealthTrendReport | null;
  range: TrendRange;
  isLoading: boolean;
  loadError: string | null;
  setRange: (range: TrendRange) => void;
  loadHealthTrend: (range?: TrendRange) => Promise<void>;
}

export const useHealthStore = create<HealthStore>((set, get) => ({
  trendReport: null,
  range: "days30",
  isLoading: false,
  loadError: null,
  setRange: (range) => set({ range }),
  loadHealthTrend: async (range) => {
    const nextRange = range ?? get().range;
    set({ isLoading: true, loadError: null, range: nextRange });
    try {
      const trendReport = await getHealthTrend(nextRange);
      set({ trendReport, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
}));
