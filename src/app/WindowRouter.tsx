import { lazy, Suspense } from "react";
import type { ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ErrorBoundary } from "../components/ErrorBoundary";

/**
 * Recovery action for the compact overlay windows: re-fetch settings (the one
 * store every overlay reads) so a stale/invalid settings object can be repaired
 * on retry. Other stores (hardware/workshop) are event-fed and self-heal on the
 * next broadcast, so we don't force-reload them here.
 */
function reloadOverlayStores() {
  void import("../stores/settingsStore").then(({ useSettingsStore }) =>
    useSettingsStore.getState().loadSettings(),
  );
}

/**
 * Recovery action for the main window: reload the data stores its pages render
 * from, so a "重试" after a data-driven render crash can actually repair the
 * underlying data instead of re-rendering the same broken snapshot.
 */
function reloadMainWindowStores() {
  void Promise.all([
    import("../stores/settingsStore"),
    import("../stores/notesStore"),
    import("../stores/workshopStore"),
    import("../stores/achievementStore"),
  ]).then(
    ([
      { useSettingsStore },
      { useNotesStore },
      { useWorkshopStore },
      { useAchievementStore },
    ]) => {
      void useSettingsStore.getState().loadSettings();
      void useNotesStore.getState().loadNotes();
      void useWorkshopStore.getState().loadWorkshopState();
      void useAchievementStore.getState().loadSummary();
      void useAchievementStore.getState().loadWeeklyGoals();
    },
  );
}

/** ErrorBoundary variant that renders a compact recovery UI sized for overlay
 *  windows. Uses the `compact` prop (not `fallback`) so the retry button — which
 *  triggers `onReset` → reloadOverlayStores — is still rendered, keeping the
 *  overlay recoverable instead of permanently stuck on "渲染异常". */
function CompactErrorBoundary({ children }: { children: ReactNode }) {
  return (
    <ErrorBoundary compact onReset={reloadOverlayStores}>
      {children}
    </ErrorBoundary>
  );
}

type WindowLabel = "main" | "pet" | "monitor-bar" | "pet-panel" | "taskbar-monitor";

const MainWindow = lazy(() =>
  import("../windows/main/MainWindow").then((module) => ({
    default: module.MainWindow,
  })),
);
const MonitorBarWindow = lazy(() =>
  import("../windows/monitor-bar/MonitorBarWindow").then((module) => ({
    default: module.MonitorBarWindow,
  })),
);
const PetQuickPanelWindow = lazy(() =>
  import("../windows/pet-panel/PetQuickPanelWindow").then((module) => ({
    default: module.PetQuickPanelWindow,
  })),
);
const PetWindow = lazy(() =>
  import("../windows/pet/PetWindow").then((module) => ({
    default: module.PetWindow,
  })),
);
const TaskbarMonitorWindow = lazy(() =>
  import("../windows/taskbar-monitor/TaskbarMonitorWindow").then((module) => ({
    default: module.TaskbarMonitorWindow,
  })),
);

export function getWindowLabel(): WindowLabel {
  try {
    const label = getCurrentWindow().label;
    if (
      label === "main" ||
      label === "pet" ||
      label === "monitor-bar" ||
      label === "pet-panel" ||
      label === "taskbar-monitor"
    ) {
      return label;
    }
  } catch {
    // Browser fallback for Vite preview outside Tauri.
  }

  const pathLabel = window.location.pathname.replace("/", "");
  if (
    pathLabel === "pet" ||
    pathLabel === "monitor-bar" ||
    pathLabel === "pet-panel" ||
    pathLabel === "taskbar-monitor"
  ) {
    return pathLabel;
  }

  return "main";
}

export function WindowRouter() {
  const label = getWindowLabel();

  if (label === "pet") {
    return (
      <Suspense fallback={null}>
        <CompactErrorBoundary>
          <PetWindow />
        </CompactErrorBoundary>
      </Suspense>
    );
  }

  if (label === "monitor-bar") {
    return (
      <Suspense fallback={null}>
        <CompactErrorBoundary>
          <MonitorBarWindow />
        </CompactErrorBoundary>
      </Suspense>
    );
  }

  if (label === "pet-panel") {
    return (
      <Suspense fallback={null}>
        <CompactErrorBoundary>
          <PetQuickPanelWindow />
        </CompactErrorBoundary>
      </Suspense>
    );
  }

  if (label === "taskbar-monitor") {
    return (
      <Suspense fallback={null}>
        <CompactErrorBoundary>
          <TaskbarMonitorWindow />
        </CompactErrorBoundary>
      </Suspense>
    );
  }

  // MainWindow already wraps its page area in its own ErrorBoundary; an outer
  // one here is a belt-and-suspenders catch for any error thrown before that.
  // On retry we reload the main window's primary data stores so a render error
  // caused by stale/corrupt data can actually recover instead of looping.
  return (
    <Suspense fallback={null}>
      <ErrorBoundary onReset={reloadMainWindowStores}>
        <MainWindow />
      </ErrorBoundary>
    </Suspense>
  );
}
