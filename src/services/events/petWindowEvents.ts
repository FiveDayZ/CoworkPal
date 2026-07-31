import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  continuousWorkBreakMs,
  deriveCatStatusFromHardware,
  focusNudgeHoldMs,
  isTemperatureAboveEnterThreshold,
  isTemperatureBelowExitThreshold,
  shouldApplyCatStateTransition,
} from "../catStateRules";
import { getLastUserActivityAt } from "../userActivityTracker";
import { useFocusStore } from "../../stores/focusStore";
import { useHardwareStore } from "../../stores/hardwareStore";
import { usePetStore } from "../../stores/petStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { MemoryReleaseResult } from "../tauriCommands";
import type { FocusSessionBook } from "../../types/focus";
import type { HardwareMetricsSnapshot } from "../../types/hardware";
import type { CatState } from "../../types/pet";
import type { AppSettings } from "../../types/settings";
import {
  cleanupEventListeners,
  emptyEventCleanup,
  isTauriRuntime,
} from "./eventUtils";
import { registerPetStateListeners } from "./petStateEvents";

let lastDerivedCatState: CatState = "Idle";
let lastDerivedChangedAt = 0;
let pendingCleanupSucceeded = false;
let temperatureSafeSince: number | null = null;
let continuousWorkSince: number | null = null;
let focusNudgeState: Extract<CatState, "NeedsBreak" | "Fatigued"> | null = null;
let focusNudgeUntil: number | null = null;

export function registerPetWindowEvents() {
  const unlisteners: Array<Promise<UnlistenFn>> = [];

  if (!isTauriRuntime()) {
    return emptyEventCleanup();
  }

  unlisteners.push(
    listen<HardwareMetricsSnapshot>("hardware:metrics", (event) => {
      useHardwareStore.getState().setMetrics(event.payload);
      applyHardwareDerivedPetFallback(event.payload);
    }),
  );

  registerPetStateListeners(unlisteners);

  unlisteners.push(
    listen<FocusSessionBook>("focus:session-updated", (event) => {
      const previousActive = useFocusStore.getState().activeSession;
      useFocusStore.getState().setBook(event.payload);
      const nextActive = useFocusStore.getState().activeSession;

      if (nextActive) {
        focusNudgeState = null;
        focusNudgeUntil = null;
        return;
      }

      if (!previousActive) {
        return;
      }

      const ended = event.payload.sessions.find(
        (session) => session.id === previousActive.id,
      );
      if (ended?.status === "completed") {
        focusNudgeState = "NeedsBreak";
        focusNudgeUntil = Date.now() + focusNudgeHoldMs;
      } else if (ended?.status === "abandoned") {
        focusNudgeState = "Fatigued";
        focusNudgeUntil = Date.now() + focusNudgeHoldMs;
      }
    }),
  );

  unlisteners.push(
    listen("cleanup:succeeded", () => {
      // NOTE: this is intentionally a SECOND listener for the same event that
      // registerPetStateListeners (above) also handles. That one renders the
      // immediate Celebrate reaction + notification; this one only latches a
      // flag so the NEXT hardware-derived state pass can fold "cleanup just
      // succeeded" into its derivation (catStateRules returns Celebrate when
      // cleanupSucceeded is set). They serve different layers, not a duplicate.
      pendingCleanupSucceeded = true;
    }),
  );

  unlisteners.push(
    listen<AppSettings>("settings:updated", (event) => {
      useSettingsStore.getState().setSettings(event.payload);
    }),
  );

  unlisteners.push(
    listen<MemoryReleaseResult>("memory:release-completed", (event) => {
      // Surface the release result in the CoreCat speech bubble. The bubble's
      // visibility is gated by `enablePetBubble` in PetWindow; updating
      // catMessage here retriggers its show/hide timer so the user sees how
      // much memory was freed. Runs in the pet window's own JS context so the
      // store update is visible to the bubble renderer.
      usePetStore.getState().setCatMessage(event.payload.note);
    }),
  );

  return cleanupEventListeners(unlisteners);
}

function applyHardwareDerivedPetFallback(snapshot: HardwareMetricsSnapshot) {
  const settings = useSettingsStore.getState().settings;
  if (!settings) {
    return;
  }

  const nextSafeSince = nextTemperatureSafeSince(
    snapshot,
    settings,
    lastDerivedCatState,
    temperatureSafeSince,
  );
  const lastUserActivityAt = getLastUserActivityAt();
  const activeFocusSession = useFocusStore.getState().activeSession;
  continuousWorkSince = nextContinuousWorkSince(
    snapshot.timestamp,
    lastUserActivityAt,
    continuousWorkSince,
  );
  const event = deriveCatStatusFromHardware(snapshot, settings, {
    activeFocusPlannedDurationSeconds:
      activeFocusSession?.plannedDurationSeconds ?? null,
    activeFocusStartedAt: activeFocusSession?.startedAt ?? null,
    cleanupSucceeded: pendingCleanupSucceeded,
    continuousWorkSince,
    currentCatState: lastDerivedCatState,
    focusNudgeState,
    focusNudgeUntil,
    lastUserActivityAt,
    temperatureSafeSince: nextSafeSince,
    timestamp: snapshot.timestamp,
  });
  pendingCleanupSucceeded = false;
  temperatureSafeSince = nextSafeSince;
  const elapsedMs = event.timestamp - lastDerivedChangedAt;

  if (
    lastDerivedChangedAt > 0 &&
    !shouldApplyCatStateTransition(
      lastDerivedCatState,
      event.catState,
      elapsedMs,
    )
  ) {
    return;
  }

  lastDerivedCatState = event.catState;
  lastDerivedChangedAt = event.timestamp;
  usePetStore.getState().setPetStatus(event);
}

function nextTemperatureSafeSince(
  snapshot: HardwareMetricsSnapshot,
  settings: Pick<AppSettings, "cpuTemperatureWarning" | "gpuTemperatureWarning">,
  currentCatState: CatState,
  currentSafeSince: number | null,
) {
  if (isTemperatureAboveEnterThreshold(snapshot, settings)) {
    return null;
  }

  if (
    currentCatState === "TemperatureCheck" &&
    isTemperatureBelowExitThreshold(snapshot)
  ) {
    return currentSafeSince ?? snapshot.timestamp;
  }

  return null;
}

function nextContinuousWorkSince(
  timestamp: number,
  lastUserActivityAt: number | null,
  currentSince: number | null,
) {
  if (
    lastUserActivityAt != null &&
    timestamp - lastUserActivityAt < continuousWorkBreakMs
  ) {
    return currentSince ?? timestamp;
  }

  return null;
}
