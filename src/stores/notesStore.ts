import { create } from "zustand";
import {
  createNote as createNoteCmd,
  deleteNote as deleteNoteCmd,
  exportNote as exportNoteCmd,
  getNotes,
  importNote as importNoteCmd,
  toggleNoteArchived as toggleArchivedCmd,
  toggleNotePinned as togglePinnedCmd,
  updateNote as updateNoteCmd,
} from "../services/tauriCommands";
import type {
  NoteBook,
  NoteColor,
  NoteKind,
  NotesFilter,
} from "../types/notes";

/**
 * Modal interaction model: the list page never reads/writes note content
 * inline. Instead, viewing and editing are always done in dedicated modals.
 *
 *  - `viewId`    → the read-only view modal is open for this note id (null = closed)
 *  - `editTarget` → the edit modal is open, either for an existing note id or
 *                  `"new"` for a brand-new note (null = closed). Only one of
 *                  view/edit is open at a time; opening edit from the view
 *                  modal closes the view.
 */
export type EditTarget = string | "new" | null;

/** When opening "new" edit, which kind to preselect. */
let pendingNewKind: NoteKind = "note";

// Monotonic sequence for last-writer-wins on the note book. Each mutating
// action (create/update/toggle/delete) increments this and only commits its
// returned book if it is still the latest. Without this, a slow earlier
// response could land after a faster later one and overwrite the newer state
// ("过期响应覆盖最新"). Reads (loadNotes/getNotes) are not sequenced — they are
// only triggered on demand and always reflect the freshest snapshot.
let notesMutationSeq = 0;

export interface NotesStore {
  book: NoteBook | null;
  filter: NotesFilter;
  viewId: string | null;
  editTarget: EditTarget;
  newKind: NoteKind;
  isLoading: boolean;
  loadError: string | null;

  setBook: (book: NoteBook) => void;
  setFilter: (filter: NotesFilter) => void;

  /** Open the read-only view modal for a note. */
  openView: (id: string) => void;
  closeView: () => void;
  /** Open the edit modal for an existing note, or for a new note of `kind`. */
  openEditExisting: (id: string) => void;
  openEditNew: (kind: NoteKind) => void;
  closeEdit: () => void;

  loadNotes: () => Promise<void>;
  createNote: (args: {
    kind: NoteKind;
    title: string;
    body: string;
    memoDueAt: number | null;
    color: NoteColor;
  }) => Promise<string | null>;
  updateNote: (args: {
    id: string;
    title: string;
    body: string;
    memoDueAt: number | null;
    color: NoteColor;
  }) => Promise<void>;
  togglePinned: (id: string) => Promise<void>;
  toggleArchived: (id: string) => Promise<void>;
  deleteNote: (id: string) => Promise<void>;
  /** Export a note as .md; returns the saved path or null if cancelled. */
  exportNote: (id: string) => Promise<string | null>;
  /** Import a .md file as a new note; returns the new id or null if cancelled. */
  importNote: () => Promise<string | null>;
}

export const useNotesStore = create<NotesStore>((set, get) => ({
  book: null,
  filter: "all",
  viewId: null,
  editTarget: null,
  newKind: "note",
  isLoading: false,
  loadError: null,

  setBook: (book) =>
    set((state) => {
      // Event-path guard: the `notes:updated` broadcast can arrive out of order
      // relative to local mutations / IPC responses. Drop a stale event only
      // when it cannot be newer: same note count but an older max updatedAt
      // means it's an update from before the current state. When the count
      // differs (a create or delete happened) we always accept, since
      // structural changes can't be ranked by timestamp alone (a delete lowers
      // the count without advancing any note's updatedAt).
      if (state.book && state.book.notes.length === book.notes.length) {
        const currentMax = Math.max(0, ...state.book.notes.map((n) => n.updatedAt));
        const incomingMax = Math.max(0, ...book.notes.map((n) => n.updatedAt));
        if (incomingMax < currentMax) {
          return state;
        }
      }
      return { book };
    }),
  setFilter: (filter) => set({ filter }),

  openView: (id) => set({ viewId: id, editTarget: null }),
  closeView: () => set({ viewId: null }),
  openEditExisting: (id) => set({ editTarget: id, viewId: null }),
  openEditNew: (kind) => {
    pendingNewKind = kind;
    set({ editTarget: "new", viewId: null, newKind: kind });
  },
  closeEdit: () => set({ editTarget: null }),

  loadNotes: async () => {
    set({ isLoading: true, loadError: null });
    try {
      const book = await getNotes();
      // Route through setBook (not a bare set) so the freshness guard applies:
      // if a concurrent write landed a newer book while this fetch was in
      // flight, we won't clobber it with this stale snapshot.
      get().setBook(book);
      set({ isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
  createNote: async (args) => {
    set({ isLoading: true, loadError: null });
    const seq = ++notesMutationSeq;
    try {
      // Backend now returns [book, newId] — no more fragile title-based lookup.
      const [book, newId] = await createNoteCmd(args);
      if (seq === notesMutationSeq) {
        // Route through setBook (freshness guard) instead of a bare set: even
        // though seq says this is our latest mutation, a newer cross-window
        // event may have landed a fresher book while this IPC was in flight.
        get().setBook(book);
      }
      return newId;
    } catch (error) {
      if (seq === notesMutationSeq) {
        set({
          loadError: error instanceof Error ? error.message : String(error),
        });
      }
      // Rethrow so the editor modal's catch can surface the failure to the user
      // (it keeps the modal open + shows the error). Previously the error was
      // swallowed here, which made the modal close on failure as if it saved.
      throw error;
    } finally {
      // isLoading is set only by createNote, so it must be cleared here
      // regardless of seq — previously it was cleared only when seq was still
      // latest, which left isLoading stuck true if another mutation overtook
      // this one while it was in flight (UI stuck on "loading").
      set({ isLoading: false });
    }
  },
  updateNote: async (args) => {
    set({ loadError: null });
    const seq = ++notesMutationSeq;
    try {
      const book = await updateNoteCmd(args);
      if (seq === notesMutationSeq) {
        get().setBook(book);
      }
    } catch (error) {
      if (seq === notesMutationSeq) {
        set({
          loadError: error instanceof Error ? error.message : String(error),
        });
      }
      // Rethrow: the editor modal must learn the save failed so it stays open.
      throw error;
    }
  },
  togglePinned: async (id) => {
    const seq = ++notesMutationSeq;
    try {
      const book = await togglePinnedCmd(id);
      if (seq === notesMutationSeq) {
        get().setBook(book);
      }
    } catch (error) {
      if (seq === notesMutationSeq) {
        set({
          loadError: error instanceof Error ? error.message : String(error),
        });
      }
      throw error;
    }
  },
  toggleArchived: async (id) => {
    const seq = ++notesMutationSeq;
    try {
      const book = await toggleArchivedCmd(id);
      if (seq === notesMutationSeq) {
        get().setBook(book);
      }
    } catch (error) {
      if (seq === notesMutationSeq) {
        set({
          loadError: error instanceof Error ? error.message : String(error),
        });
      }
      throw error;
    }
  },
  deleteNote: async (id) => {
    const seq = ++notesMutationSeq;
    try {
      const book = await deleteNoteCmd(id);
      if (seq === notesMutationSeq) {
        get().setBook(book);
      }
    } catch (error) {
      if (seq === notesMutationSeq) {
        set({
          loadError: error instanceof Error ? error.message : String(error),
        });
      }
      // Rethrow: the view modal awaits this before closing — without rethrow,
      // a failed delete would close the modal and hide the failure.
      throw error;
    }
  },
  exportNote: async (id) => {
    try {
      return await exportNoteCmd(id);
    } catch (error) {
      set({
        loadError: error instanceof Error ? error.message : String(error),
      });
      // Rethrow so the caller (view modal) can show its inline toast instead
      // of silently treating a failed export as a user-cancelled export.
      throw error;
    }
  },
  importNote: async () => {
    const seq = ++notesMutationSeq;
    try {
      // Backend shows the open dialog, reads the file, creates the note, and
      // emits notes:updated. The returned id lets the caller (e.g. open the
      // new note for editing) act on it; null means the user cancelled.
      const newId = await importNoteCmd();
      if (newId) {
        // The backend emits NOTES_UPDATED, but route through setBook with the
        // seq guard so an in-flight local mutation isn't clobbered. We don't
        // have the book here (backend owns it), so rely on the event listener
        // + a defensive reload if seq is still current.
        if (seq === notesMutationSeq) {
          void get().loadNotes();
        }
      }
      return newId;
    } catch (error) {
      if (seq === notesMutationSeq) {
        set({
          loadError: error instanceof Error ? error.message : String(error),
        });
      }
      throw error;
    }
  },
}));
