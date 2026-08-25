import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import {
  hideMainWindow,
  showMainWindow,
  showPetPanel,
  togglePetPanel,
} from "./tauriCommands";

const MAIN_WINDOW_BASE_WIDTH = 640;
const MAIN_WINDOW_BASE_HEIGHT = 420;
const MAIN_WINDOW_ZOOM_SCALE = 2;

let mainWindowZoomed = false;

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function startDraggingCurrentWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await getCurrentWindow().startDragging();
}

export async function openMainWindow(): Promise<void> {
  await showMainWindow();
}

export async function openPetPanel(): Promise<void> {
  await showPetPanel();
}

export async function toggleQuickPanel(): Promise<void> {
  await togglePetPanel();
}

export async function minimizeMainWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }
  await getCurrentWindow().minimize();
}

export function isMainWindowZoomed(): boolean {
  return mainWindowZoomed;
}

export async function toggleMainWindowZoom(): Promise<boolean> {
  const nextZoomed = !mainWindowZoomed;
  const scale = nextZoomed ? MAIN_WINDOW_ZOOM_SCALE : 1;

  if (isTauriRuntime()) {
    await getCurrentWindow().setSize(
      new LogicalSize(
        MAIN_WINDOW_BASE_WIDTH * scale,
        MAIN_WINDOW_BASE_HEIGHT * scale,
      ),
    );
  }

  mainWindowZoomed = nextZoomed;
  return mainWindowZoomed;
}

export async function closeMainWindow(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }
  await hideMainWindow();
}
