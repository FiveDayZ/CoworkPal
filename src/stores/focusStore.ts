import { create } from "zustand";
import type { FocusSessionBook, FocusSession } from "../types/focus";
import {
  abandonFocusSession,
  completeFocusSession,
  getFocusSessions,
  startFocusSession,
} from "../services/tauriCommands";
import { useWorkshopStore } from "./workshopStore";

export interface FocusStore {
  book: FocusSessionBook | null;
  /** Convenience: the single active session, if any. */
  activeSession: FocusSession | null;
  isLoading: boolean;
  isStarting: boolean;
  isEnding: boolean;
  loadError: string | null;
  setBook: (book: FocusSessionBook) => void;
  load: () => Promise<void>;
  start: (taskLabel: string, durationMinutes: number) => Promise<void>;
  complete: () => Promise<void>;
  abandon: () => Promise<void>;
}

function deriveActive(book: FocusSessionBook | null): FocusSession | null {
  if (!book) {
    return null;
  }
  return book.sessions.reduce<FocusSession | null>(
    (latest, session) =>
      session.status === "active" && (!latest || session.startedAt > latest.startedAt)
        ? session
        : latest,
    null,
  );
}

export const useFocusStore = create<FocusStore>((set, get) => ({
  book: null,
  activeSession: null,
  isLoading: false,
  isStarting: false,
  isEnding: false,
  loadError: null,
  setBook: (book) => set({ book, activeSession: deriveActive(book) }),
  load: async () => {
    const bookAtRequest = get().book;
    set({ isLoading: true, loadError: null });
    try {
      const book = await getFocusSessions();
      if (get().book === bookAtRequest) {
        set({ book, activeSession: deriveActive(book), isLoading: false });
      } else {
        set({ isLoading: false });
      }
    } catch (error) {
      set({
        isLoading: false,
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
  start: async (taskLabel, durationMinutes) => {
    if (get().isStarting) {
      return;
    }
    set({ isStarting: true, loadError: null });
    try {
      const book = await startFocusSession(taskLabel, durationMinutes);
      set({ book, activeSession: deriveActive(book), isStarting: false });
    } catch (error) {
      set({ isStarting: false });
      throw error;
    }
  },
  complete: async () => {
    const { activeSession, isEnding } = get();
    if (!activeSession || isEnding) {
      return;
    }
    set({ isEnding: true });
    try {
      const [book, workshop] = await completeFocusSession(activeSession.id);
      set({ book, activeSession: deriveActive(book) });
      // completeFocusSession lands workshop rewards; sync the workshop store.
      useWorkshopStore.getState().setWorkshopState(workshop);
    } catch (error) {
      // The session remains active on failure (backend rolls back), so we only
      // log here; the caller (pet panel) keeps the user in the active session.
      console.error("Failed to complete focus session", error);
      throw error;
    } finally {
      set({ isEnding: false });
    }
  },
  abandon: async () => {
    const { activeSession, isEnding } = get();
    if (!activeSession || isEnding) {
      return;
    }
    set({ isEnding: true });
    try {
      const book = await abandonFocusSession(activeSession.id);
      set({ book, activeSession: deriveActive(book) });
    } catch (error) {
      console.error("Failed to abandon focus session", error);
      throw error;
    } finally {
      set({ isEnding: false });
    }
  },
}));
