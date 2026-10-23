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

// Monotonic sequence used to implement last-writer-wins. Every optimistic
// update gets an id; when the backend response comes back we only commit it
// if it is still the latest request. This prevents an earlier request's slow
// response from clobbering a newer value that has already been applied.
let pendingRequestSeq = 0;
let settingsWriteQueue: Promise<unknown> = Promise.resolve();

// Accumulates optimistic patches applied locally but whose IPC round-trip has
// not resolved. The `settings:updated` broadcast event re-applies this on top
// of its payload so an in-flight local edit (e.g. a slider drag) is preserved
// instead of being overwritten by a stale backend snapshot.
//
// Queued writes carry all pending fields; only the latest response clears them.
let pendingOptimisticPatch: AppSettingsPatch = {};

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: null,
  setSettings: (settings) =>
    set((state) => {
      // Event-path guard: if there are local optimistic edits still in flight,
      // preserve them by re-applying the pending patch on top of the incoming
      // backend snapshot.
      if (state.settings && Object.keys(pendingOptimisticPatch).length > 0) {
        return { settings: { ...settings, ...pendingOptimisticPatch } };
      }
      return { settings };
    }),
  applyOptimistic: (patch) => {
    ++pendingRequestSeq;
    const previousSettings = get().settings;
    if (previousSettings) {
      set({ settings: { ...previousSettings, ...patch } });
    }
    Object.assign(pendingOptimisticPatch, patch);
  },
  loadSettings: async () => {
    const settings = await getAppSettings();
    get().setSettings(settings);
  },
  updateSettings: async (patch) => {
    const seq = ++pendingRequestSeq;
    Object.assign(pendingOptimisticPatch, patch);
    const committedPatch = { ...pendingOptimisticPatch };
    const request = settingsWriteQueue.then(() => updateAppSettings(committedPatch));
    settingsWriteQueue = request.catch(() => {});

    // Apply the patch optimistically so the UI reacts immediately. On failure
    // we re-fetch authoritative state rather than rolling back to a snapshot
    // (which may already have been mutated by a newer optimistic update).
    const current = get().settings;
    if (current) {
      set({ settings: { ...current, ...patch } });
    }

    try {
      const settings = await request;
      // Only apply the authoritative response if no newer request has started.
      if (seq === pendingRequestSeq) {
        // The latest request resolved — all edits up to here are committed, so
        // the pending optimistic set no longer needs event-path protection.
        pendingOptimisticPatch = {};
        set({ settings });
      }
    } catch (error) {
      if (seq === pendingRequestSeq) {
        try {
          const authoritative = await getAppSettings();
          if (seq === pendingRequestSeq) {
            pendingOptimisticPatch = {};
            set({ settings: authoritative });
          }
        } catch {
          // Re-fetch failed: leave optimistic value; next event/load reconciles.
        }
      }
      throw error;
    }
  },
}));
