import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { usePetStore } from "../../stores/petStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { CatStateChangedEvent } from "../../types/pet";

let lastPetState: CatStateChangedEvent | null = null;

export function registerPetStateListeners(
  unlisteners: Array<Promise<UnlistenFn>>,
) {
  unlisteners.push(
    listen<CatStateChangedEvent>("pet:state-changed", (event) => {
      lastPetState = event.payload;
      usePetStore.getState().setPetStatus(event.payload);
      maybeNotifyPetState(event.payload);
    }),
  );

  unlisteners.push(
    listen("cleanup:succeeded", () => {
      applyCleanupSucceeded(Date.now());
    }),
  );
}

function applyCleanupSucceeded(timestamp: number) {
  const event: CatStateChangedEvent = {
    timestamp,
    catState: "Celebrate",
    catMessage: "清理完成，工坊状态良好。",
  };
  lastPetState = event;
  usePetStore.getState().setPetStatus(event);
  maybeNotify("CoCat", event.catMessage);
}

function maybeNotifyPetState(event: CatStateChangedEvent) {
  if (
    event.catState !== "TemperatureCheck" &&
    event.catState !== "MemoryCrowded" &&
    event.catState !== "RepairHeavy"
  ) {
    return;
  }

  maybeNotify("CoCat", event.catMessage);
}

function maybeNotify(title: string, body: string) {
  const settings = useSettingsStore.getState().settings;
  if (!settings?.enableNotifications || typeof window === "undefined") {
    return;
  }

  if (!("Notification" in window)) {
    return;
  }

  if (Notification.permission === "granted") {
    new Notification(title, { body });
    return;
  }

  if (Notification.permission === "default") {
    void Notification.requestPermission().then((permission) => {
      if (permission === "granted") {
        new Notification(title, { body });
      }
    });
  }
}

export function getLastPetState() {
  return lastPetState;
}
