import { create } from "zustand";
import type {
  HardwareDeviceInventory,
  HardwareMetricsSnapshot,
  HardwareSnapshot,
} from "../types/hardware";

export interface HardwareStore {
  snapshot: HardwareSnapshot | null;
  setMetrics: (snapshot: HardwareMetricsSnapshot) => void;
  setSnapshot: (snapshot: HardwareSnapshot) => void;
}

const emptyDeviceInventory: HardwareDeviceInventory = {
  motherboard: [],
  memoryModules: [],
  gpus: [],
  displays: [],
  disks: [],
  audioDevices: [],
  networkAdapters: [],
};

export const useHardwareStore = create<HardwareStore>((set) => ({
  snapshot: null,
  setMetrics: (metrics) =>
    set((state) => ({
      snapshot: {
        ...metrics,
        deviceInventory:
          state.snapshot?.deviceInventory ?? emptyDeviceInventory,
      },
    })),
  setSnapshot: (snapshot) => set({ snapshot }),
}));
