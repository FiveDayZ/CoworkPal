import { create } from "zustand";
import type { FocusSessionBook, FocusSession } from "../types/focus";
import {
  abandonFocusSession,
  completeFocusSession,
  getFocusSessions,
  startFocusSession,
  updateFocusOutcome,
} from "../services/tauriCommands";
import { useWorkshopStore } from "./workshopStore";

export interface FocusStore {
  book: FocusSessionBook | null;
  draftNote: { id: string; title: string } | null;
  setDraftNote: (note: { id: string; title: string } | null) => void;
  saveOutcome: (sessionId: string, outcome: string) => Promise<void>;
  /** Convenience: the single active session, if any. */
  activeSession: FocusSession | null;
  isLoading: boolean;
  isStarting: boolean;
  isEnding: boolean;
  loadError: string | null;
  setBook: (book: FocusSessionBook) => void;
  setActiveTick: (session: FocusSession) => void;
  load: () => Promise<void>;
  start: (taskLabel: string, durationMinutes: number, noteId?: string | null) => Promise<void>;
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
  draftNote: null,
  setDraftNote: (draftNote) => set({ draftNote }),
  saveOutcome: async (sessionId, outcome) => {
    const book = await updateFocusOutcome(sessionId, outcome);
    get().setBook(book);
  },
  activeSession: null,
  isLoading: false,
  isStarting: false,
  isEnding: false,
  loadError: null,
  setBook: (book) => set({ book, activeSession: deriveActive(book) }),
  setActiveTick: (session) => set((state) => {
    const saved = state.book?.sessions.find((item) => item.id === session.id);
    if (saved && saved.status !== "active") return {};
    if (state.activeSession && (state.activeSession.startedAt > session.startedAt
      || (state.activeSession.lastTickAt ?? 0) > (session.lastTickAt ?? 0))) return {};
    return { activeSession: session };
  }),
  load: async () => {
    const bookAtRequest = get().book;
    const activeAtRequest = get().activeSession;
    set({ isLoading: true, loadError: null });
    try {
      const book = await getFocusSessions();
      if (get().book === bookAtRequest) {
        const active = deriveActive(book);
        const tick = get().activeSession;
        const saved = tick && book.sessions.find((session) => session.id === tick.id);
        const keepTick = tick && tick !== activeAtRequest && (!saved || saved.status === "active")
          && (!active || tick.startedAt >= active.startedAt)
          && (tick.lastTickAt ?? 0) >= (active?.lastTickAt ?? 0);
        set({ book, activeSession: keepTick ? tick : active, isLoading: false });
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
  start: async (taskLabel, durationMinutes, noteId = null) => {
    if (get().isStarting) {
      return;
    }
    set({ isStarting: true, loadError: null });
    try {
      const book = await startFocusSession(taskLabel, durationMinutes, noteId);
      set({ draftNote: null });
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
      // Completion can be saved while payment remains pending for backend retry.
      await get().load();
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
