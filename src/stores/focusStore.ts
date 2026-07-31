import { create } from "zustand";
import type { FocusSessionBook, FocusSession } from "../types/focus";
import {
  abandonFocusSession,
  completeFocusSession,
  startFocusSession,
} from "../services/tauriCommands";
import { useWorkshopStore } from "./workshopStore";

export interface FocusStore {
  book: FocusSessionBook | null;
  /** Convenience: the single active session, if any. */
  activeSession: FocusSession | null;
  isStarting: boolean;
  setBook: (book: FocusSessionBook) => void;
  start: (taskLabel: string, durationMinutes: number) => Promise<void>;
  complete: () => Promise<void>;
  abandon: () => Promise<void>;
}

function deriveActive(book: FocusSessionBook | null): FocusSession | null {
  if (!book) {
    return null;
  }
  return book.sessions.find((s) => s.status === "active") ?? null;
}

export const useFocusStore = create<FocusStore>((set, get) => ({
  book: null,
  activeSession: null,
  isStarting: false,
  setBook: (book) => set({ book, activeSession: deriveActive(book) }),
  start: async (taskLabel, durationMinutes) => {
    set({ isStarting: true });
    try {
      const book = await startFocusSession(taskLabel, durationMinutes);
      set({ book, activeSession: deriveActive(book), isStarting: false });
    } catch (error) {
      set({ isStarting: false });
      throw error;
    }
  },
  complete: async () => {
    const { activeSession } = get();
    if (!activeSession) {
      return;
    }
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
    }
  },
  abandon: async () => {
    const { activeSession } = get();
    if (!activeSession) {
      return;
    }
    try {
      const book = await abandonFocusSession(activeSession.id);
      set({ book, activeSession: deriveActive(book) });
    } catch (error) {
      console.error("Failed to abandon focus session", error);
      throw error;
    }
  },
}));
