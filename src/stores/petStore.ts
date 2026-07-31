import { create } from "zustand";
import type { CatState, CatStateChangedEvent } from "../types/pet";

export interface PetStore {
  catState: CatState;
  catMessage: string;
  isPanelOpen: boolean;
  setCatState: (state: CatState) => void;
  setCatMessage: (message: string) => void;
  setPetStatus: (event: CatStateChangedEvent) => void;
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
    set({ catState: event.catState, catMessage: event.catMessage }),
  openPanel: () => set({ isPanelOpen: true }),
  closePanel: () => set({ isPanelOpen: false }),
  togglePanel: () => set((state) => ({ isPanelOpen: !state.isPanelOpen })),
}));
