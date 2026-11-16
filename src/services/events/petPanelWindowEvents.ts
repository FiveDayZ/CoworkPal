import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useFocusStore } from "../../stores/focusStore";
import { useHardwareStore } from "../../stores/hardwareStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import type { FocusSession, FocusSessionBook } from "../../types/focus";
import type { HardwareMetricsSnapshot } from "../../types/hardware";
import type { AppSettings } from "../../types/settings";
import type { WorkshopState, WorkshopTick } from "../../types/workshop";
import {
  cleanupEventListeners,
  emptyEventCleanup,
  isTauriRuntime,
} from "./eventUtils";
import { registerPetStateListeners } from "./petStateEvents";

export function registerPetPanelWindowEvents() {
  const unlisteners: Array<Promise<UnlistenFn>> = [];

  if (!isTauriRuntime()) {
    return emptyEventCleanup();
  }

  unlisteners.push(
    listen<HardwareMetricsSnapshot>("hardware:metrics", (event) => {
      useHardwareStore.getState().setMetrics(event.payload);
    }),
  );

  unlisteners.push(
    listen<WorkshopTick>("workshop:tick", (event) => useWorkshopStore.getState().setTick(event.payload)),
  );

  unlisteners.push(
    listen<WorkshopState>("workshop:updated", (event) => {
      useWorkshopStore.getState().setWorkshopState(event.payload);
    }),
  );

  unlisteners.push(
    listen<FocusSession>("focus:session-tick", (event) => {
      useFocusStore.getState().setActiveTick(event.payload);
    }),
  );

  unlisteners.push(
    listen<FocusSessionBook>("focus:session-updated", (event) => {
      useFocusStore.getState().setBook(event.payload);
    }),
  );

  registerPetStateListeners(unlisteners);

  unlisteners.push(
    listen<AppSettings>("settings:updated", (event) => {
      useSettingsStore.getState().setSettings(event.payload);
    }),
  );

  return cleanupEventListeners(unlisteners);
}
