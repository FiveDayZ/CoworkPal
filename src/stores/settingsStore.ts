import { create } from "zustand";
import { getAppSettings, updateAppSettings } from "../services/tauriCommands";
import type { AppSettings, AppSettingsPatch } from "../types/settings";

export interface SettingsStore {
  settings: AppSettings | null;
  setSettings: (settings: AppSettings) => void;
  /** Update local state only, without persisting. Used by debounced sliders. */
  applyOptimistic: (patch: AppSettingsPatch) => void;
  loadSettings: () => Promise<void>;
  updateSettings: (patch: AppSettingsPatch) => Promise<void>;
}

let settingsWriteQueue: Promise<unknown> = Promise.resolve();
let authoritativeSettings: AppSettings | null = null;
let pendingRequestSeq = 0;
const pendingFieldSeq = new Map<keyof AppSettings, number>();
const pendingOptimisticPatch: AppSettingsPatch = {};

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: null,
  setSettings: (settings) => {
    authoritativeSettings = settings;
    set({ settings: { ...settings, ...pendingOptimisticPatch } });
  },
  applyOptimistic: (patch) => {
    const seq = ++pendingRequestSeq;
    for (const key of Object.keys(patch) as Array<keyof AppSettings>) {
      pendingFieldSeq.set(key, seq);
    }
    Object.assign(pendingOptimisticPatch, patch);
    const settings = get().settings;
    if (settings) set({ settings: { ...settings, ...patch } });
  },
  loadSettings: async () => {
    get().setSettings(await getAppSettings());
  },
  updateSettings: async (patch) => {
    get().applyOptimistic(patch);
    const seq = pendingRequestSeq;
    const committedPatch = { ...patch };
    const clearSettledFields = () => {
      for (const key of Object.keys(committedPatch) as Array<keyof AppSettings>) {
        if (pendingFieldSeq.get(key) === seq) {
          pendingFieldSeq.delete(key);
          delete pendingOptimisticPatch[key];
        }
      }
    };
    const request = settingsWriteQueue.then(async () => {
      try {
        const settings = await updateAppSettings(committedPatch);
        clearSettledFields();
        get().setSettings(settings);
      } catch (error) {
        // Reconcile before the next write; a failed field must not poison it.
        const settings = await getAppSettings().catch(() => authoritativeSettings);
        clearSettledFields();
        if (settings) get().setSettings(settings);
        throw error;
      }
    });
    settingsWriteQueue = request.catch(() => {});
    await request;
  },
}));
