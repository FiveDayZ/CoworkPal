import { create } from "zustand";
import {
  getWorkshopState,
  getWorkshopProductionBreakdown,
  getWorkshopUpgradeQuotes,
  completeWorkshopOrder,
  resetWorkshopState,
  upgradeWorkshop,
  upgradeWorkshopModule,
} from "../services/tauriCommands";
import type {
  WorkshopModuleKey,
  WorkshopProductionBreakdown,
  WorkshopState,
  WorkshopTick,
  WorkshopUpgradeQuotes,
} from "../types/workshop";

export interface WorkshopStore {
  state: WorkshopState | null;
  quotes: WorkshopUpgradeQuotes | null;
  breakdown: WorkshopProductionBreakdown | null;
  setWorkshopState: (state: WorkshopState) => void;
  setTick: (tick: WorkshopTick) => void;
  loadWorkshopState: () => Promise<void>;
  upgradeWorkshop: () => Promise<void>;
  upgradeModule: (
    moduleKey: WorkshopModuleKey,
    track: "parts" | "process",
  ) => Promise<void>;
  resetWorkshop: () => Promise<void>;
  completeOrder: (orderId: string) => Promise<void>;
  refreshBreakdown: () => Promise<void>;
}

export const useWorkshopStore = create<WorkshopStore>((set, get) => ({
  state: null,
  quotes: null,
  breakdown: null,
  setTick: (tick) => set((current) => current.state
    && (tick.revision ?? 0) === (current.state.revision ?? 0)
    && tick.lastProductionTime >= current.state.lastProductionTime
    ? { state: { ...current.state, ...tick } } : {}),
  setWorkshopState: (state) => {
    let refreshQuotes = false;
    set((current) => {
      if ((state.revision ?? 0) < (current.state?.revision ?? 0)) return {};
      if (current.state && (state.revision ?? 0) === (current.state.revision ?? 0)
        && state.lastProductionTime < current.state.lastProductionTime) return {};
      refreshQuotes = upgradeLevelsChanged(current.state, state);
      return { state, quotes: refreshQuotes ? null : current.quotes };
    });
    if (refreshQuotes) {
      void getWorkshopUpgradeQuotes()
        .then((quotes) =>
          set((current) =>
            current.state && !upgradeLevelsChanged(state, current.state) ? { quotes } : {},
          ),
        )
        .catch((error) => console.error("Failed to refresh workshop upgrade quotes", error));
    }
  },
  loadWorkshopState: async () => {
    try {
      const state = await getWorkshopState();
      get().setWorkshopState(state);
      const [quotes, breakdown] = await Promise.all([getWorkshopUpgradeQuotes(), getWorkshopProductionBreakdown()]);
      set((current) => current.state && !upgradeLevelsChanged(state, current.state) ? { quotes, breakdown } : {});
    } catch (error) {
      // Startup load failures must not become unhandled rejections; the UI
      // stays on its initial null state and the error is surfaced in console.
      console.error("Failed to load workshop state", error);
    }
  },
  upgradeWorkshop: async () => {
    const state = await upgradeWorkshop();
    get().setWorkshopState(state);
    const [quotes, breakdown] = await Promise.all([
      getWorkshopUpgradeQuotes(),
      getWorkshopProductionBreakdown(),
    ]);
    set((current) => current.state && !upgradeLevelsChanged(state, current.state) ? { quotes, breakdown } : {});
  },
  upgradeModule: async (moduleKey, track) => {
    const state = await upgradeWorkshopModule(moduleKey, track);
    get().setWorkshopState(state);
    const [quotes, breakdown] = await Promise.all([
      getWorkshopUpgradeQuotes(),
      getWorkshopProductionBreakdown(),
    ]);
    set((current) => current.state && !upgradeLevelsChanged(state, current.state) ? { quotes, breakdown } : {});
  },
  resetWorkshop: async () => {
    const state = await resetWorkshopState();
    get().setWorkshopState(state);
    const quotes = await getWorkshopUpgradeQuotes();
    set((current) => current.state && !upgradeLevelsChanged(state, current.state) ? { quotes } : {});
  },
  completeOrder: async (orderId) => {
    const state = await completeWorkshopOrder(orderId);
    get().setWorkshopState(state);
    const breakdown = await getWorkshopProductionBreakdown();
    set({ breakdown });
  },
  refreshBreakdown: async () => {
    const breakdown = await getWorkshopProductionBreakdown();
    set({ breakdown });
  },
}));

function upgradeLevelsChanged(previous: WorkshopState | null, next: WorkshopState): boolean {
  if (!previous || previous.workshopLevel !== next.workshopLevel) return true;
  return (Object.keys(next.moduleLevels) as WorkshopModuleKey[]).some(
    (key) =>
      previous.moduleLevels[key]?.parts !== next.moduleLevels[key].parts ||
      previous.moduleLevels[key]?.process !== next.moduleLevels[key].process,
  );
}
