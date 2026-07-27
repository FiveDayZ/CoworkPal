import type { EventCleanup } from "../services/events/eventUtils";
import { useSettingsStore } from "../stores/settingsStore";

type BootstrapWindowLabel =
  | "main"
  | "pet"
  | "monitor-bar"
  | "pet-panel"
  | "taskbar-monitor";

let bootstrapped = false;

export function bootstrapApp(label: BootstrapWindowLabel) {
  if (bootstrapped) {
    return undefined;
  }

  bootstrapped = true;
  let cleanup: EventCleanup | undefined;
  let disposed = false;
  void useSettingsStore.getState().loadSettings();

  void bootstrapWindow(label).then((nextCleanup) => {
    if (disposed) {
      void nextCleanup();
      return;
    }

    cleanup = nextCleanup;
  });

  return () => {
    disposed = true;
    void cleanup?.();
    bootstrapped = false;
  };
}

async function bootstrapWindow(label: BootstrapWindowLabel): Promise<EventCleanup> {
  if (label === "main") {
    const [
      { registerMainWindowEvents },
      { useAchievementStore },
      { useNotesStore },
      { useWorkshopStore },
      { useWorkLogStore },
    ] = await Promise.all([
      import("../services/events/mainWindowEvents"),
      import("../stores/achievementStore"),
      import("../stores/notesStore"),
      import("../stores/workshopStore"),
      import("../stores/workLogStore"),
    ]);

    void useAchievementStore.getState().loadSummary();
    void useNotesStore.getState().loadNotes();
    void useWorkshopStore.getState().loadWorkshopState();
    void useWorkLogStore.getState().loadWorkLogReport();
    return registerMainWindowEvents();
  }

  if (label === "pet") {
    const [
      { registerPetWindowEvents },
      { registerUserActivityTracking },
    ] = await Promise.all([
      import("../services/events/petWindowEvents"),
      import("../services/userActivityTracker"),
    ]);
    const cleanup = registerPetWindowEvents();
    const unregisterUserActivityTracking = registerUserActivityTracking();

    return async () => {
      unregisterUserActivityTracking();
      await cleanup();
    };
  }

  if (label === "pet-panel") {
    const [
      { registerPetPanelWindowEvents },
      { useWorkshopStore },
    ] = await Promise.all([
      import("../services/events/petPanelWindowEvents"),
      import("../stores/workshopStore"),
    ]);

    void useWorkshopStore.getState().loadWorkshopState();
    return registerPetPanelWindowEvents();
  }

  const { registerMonitorWindowEvents } = await import(
    "../services/events/monitorWindowEvents"
  );
  return registerMonitorWindowEvents();
}
