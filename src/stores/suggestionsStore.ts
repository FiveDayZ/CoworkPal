import { create } from "zustand";
import { getTodaySuggestions } from "../services/tauriCommands";
import type { TodaySuggestions } from "../types/suggestions";

export interface SuggestionsStore {
  today: TodaySuggestions | null;
  isLoading: boolean;
  loadError: string | null;
  loadTodaySuggestions: () => Promise<void>;
}

export const useSuggestionsStore = create<SuggestionsStore>((set) => ({
  today: null,
  isLoading: false,
  loadError: null,
  loadTodaySuggestions: async () => {
    set({ isLoading: true, loadError: null });
    try {
      const today = await getTodaySuggestions();
      set({ today, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
}));
