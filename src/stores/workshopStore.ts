import { create } from "zustand";
import { getWorkshopState, updateWorkshopState } from "../services/tauriCommands";
import type { WorkshopState } from "../types/workshop";

export interface WorkshopStore {
  state: WorkshopState | null;
  setWorkshopState: (state: WorkshopState) => void;
  loadWorkshopState: () => Promise<void>;
  saveWorkshopState: (state: WorkshopState) => Promise<void>;
}

export const useWorkshopStore = create<WorkshopStore>((set) => ({
  state: null,
  setWorkshopState: (state) => set({ state }),
  loadWorkshopState: async () => {
    try {
      const state = await getWorkshopState();
      set({ state });
    } catch (error) {
      // Startup load failures must not become unhandled rejections; the UI
      // stays on its initial null state and the error is surfaced in console.
      console.error("Failed to load workshop state", error);
    }
  },
  saveWorkshopState: async (state) => {
    const updated = await updateWorkshopState(state);
    set({ state: updated });
  },
}));
