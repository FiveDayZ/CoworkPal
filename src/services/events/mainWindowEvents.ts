import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useAchievementStore } from "../../stores/achievementStore";
import { isMainRoute } from "../../routeTypes";
import { useFocusStore } from "../../stores/focusStore";
import { useHardwareStore } from "../../stores/hardwareStore";
import { useNotesStore } from "../../stores/notesStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useUiStore } from "../../stores/uiStore";
import { useWorkLogStore } from "../../stores/workLogStore";
import { useWorkshopStore } from "../../stores/workshopStore";
import type { HardwareMetricsSnapshot } from "../../types/hardware";
import type { AchievementUnlockedEvent } from "../../types/achievement";
import type { FocusSessionBook } from "../../types/focus";
import type { NoteBook } from "../../types/notes";
import type { AppSettings } from "../../types/settings";
import type { WorkLogReport } from "../../types/workLog";
import type { WorkshopState } from "../../types/workshop";
import type { MemoryReleaseResult } from "../tauriCommands";
import {
  cleanupEventListeners,
  emptyEventCleanup,
  isTauriRuntime,
} from "./eventUtils";
import { registerPetStateListeners } from "./petStateEvents";

export function registerMainWindowEvents() {
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
    listen<WorkshopState>("workshop:updated", (event) => {
      useWorkshopStore.getState().setWorkshopState(event.payload);
    }),
  );

  unlisteners.push(
    listen<FocusSessionBook>("focus:session-updated", (event) => {
      useFocusStore.getState().setBook(event.payload);
    }),
  );

  unlisteners.push(
    listen<NoteBook>("notes:updated", (event) => {
      useNotesStore.getState().setBook(event.payload);
    }),
  );

  unlisteners.push(
    listen<WorkLogReport>("worklog:updated", (event) => {
      const workLogStore = useWorkLogStore.getState();
      if (workLogStore.selectedDate === event.payload.date) {
        workLogStore.setReport(event.payload);
      }
    }),
  );

  registerPetStateListeners(unlisteners);

  unlisteners.push(
    listen<AchievementUnlockedEvent>("achievement:unlocked", (event) => {
      const achievementStore = useAchievementStore.getState();
      achievementStore.pushUnlocked(event.payload);
      void achievementStore.loadSummary().catch((error) => {
        console.error("Failed to reload achievement summary", error);
      });
      void achievementStore.loadCards().catch((error) => {
        console.error("Failed to reload achievement cards", error);
      });
    }),
  );

  unlisteners.push(
    listen<AppSettings>("settings:updated", (event) => {
      useSettingsStore.getState().setSettings(event.payload);
    }),
  );

  unlisteners.push(
    listen<string>("ui:navigate-main", (event) => {
      if (isMainRoute(event.payload)) {
        useUiStore.getState().setMainRoute(event.payload);
      }
    }),
  );

  unlisteners.push(
    listen<MemoryReleaseResult>("memory:release-completed", (event) => {
      // Toast mirrors the pet-state notifications: only when the user has
      // enabled notifications. The settings card's "last release" line is
      // driven by the settings:updated event (the backend persists
      // memoryLastRelease), so the toast only carries the summary text.
      // The CoreCat speech bubble is updated separately in petWindowEvents
      // (each window owns its own store instance).
      maybeNotifyMemoryRelease(event.payload);
    }),
  );

  return cleanupEventListeners(unlisteners);
}

function maybeNotifyMemoryRelease(result: MemoryReleaseResult) {
  const settings = useSettingsStore.getState().settings;
  if (!settings?.enableNotifications || typeof window === "undefined") {
    return;
  }
  if (!("Notification" in window)) {
    return;
  }
  const title = result.fullTier ? "内存已全量释放" : "内存已轻量释放";
  if (Notification.permission === "granted") {
    new Notification(title, { body: result.note });
    return;
  }
  if (Notification.permission === "default") {
    void Notification.requestPermission().then((permission) => {
      if (permission === "granted") {
        new Notification(title, { body: result.note });
      }
    });
  }
}
