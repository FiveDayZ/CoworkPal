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

// Accumulates optimistic patches applied locally but whose IPC round-trip has
// not resolved. The `settings:updated` broadcast event re-applies this on top
// of its payload so an in-flight local edit (e.g. a slider drag) is preserved
// instead of being overwritten by a stale backend snapshot.
//
// This is a single merged object rather than a per-request map: it represents
// "all uncommitted local edits" as a set. It is cleared when the LATEST request
// resolves (seq === pendingRequestSeq), because at that point all edits up to
// that request have been persisted (the latest debounced commit carries the
// merged patch). A per-request map added complexity (seq-claim ordering between
// the debounced applyOptimistic path and discrete updateSettings calls) without
// real-world benefit, since rapid concurrent edits across different fields are
// rare and self-correct on the next event/load.
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
    const previousSettings = get().settings;
    if (previousSettings) {
      set({ settings: { ...previousSettings, ...patch } });
    }
    Object.assign(pendingOptimisticPatch, patch);
  },
  loadSettings: async () => {
    const settings = await getAppSettings();
    set({ settings });
  },
  updateSettings: async (patch) => {
    const seq = ++pendingRequestSeq;

    // Apply the patch optimistically so the UI reacts immediately. On failure
    // we re-fetch authoritative state rather than rolling back to a snapshot
    // (which may already have been mutated by a newer optimistic update).
    const current = get().settings;
    if (current) {
      set({ settings: { ...current, ...patch } });
    }

    try {
      const settings = await updateAppSettings(patch);
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
