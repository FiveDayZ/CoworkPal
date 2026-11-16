import { create } from "zustand";
import type { MainRoute } from "../routeTypes";

export interface UiStore {
  mainRoute: MainRoute;
  workshopTab: "map" | "review" | "display";
  setWorkshopTab: (tab: "map" | "review" | "display") => void;
  performanceHistoryOpen: boolean;
  setPerformanceHistoryOpen: (open: boolean) => void;
  setMainRoute: (route: MainRoute) => void;
}

export const useUiStore = create<UiStore>((set) => ({
  mainRoute: "dashboard",
  workshopTab: "map",
  setWorkshopTab: (workshopTab) => set({ workshopTab, performanceHistoryOpen: workshopTab === "review" }),
  performanceHistoryOpen: false,
  setPerformanceHistoryOpen: (performanceHistoryOpen) => set({ performanceHistoryOpen }),
  setMainRoute: (mainRoute) => set({ mainRoute }),
}));
