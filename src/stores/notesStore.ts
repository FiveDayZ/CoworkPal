import { create } from "zustand";
import {
  createNote as createNoteCmd,
  deleteNote as deleteNoteCmd,
  getNotes,
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
}

export const useNotesStore = create<NotesStore>((set) => ({
  book: null,
  filter: "all",
  viewId: null,
  editTarget: null,
  newKind: "note",
  isLoading: false,
  loadError: null,

  setBook: (book) => set({ book }),
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
      set({ book, isLoading: false });
    } catch (error) {
      set({
        isLoading: false,
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
  createNote: async (args) => {
    set({ isLoading: true, loadError: null });
    try {
      const book = await createNoteCmd(args);
      const created = book.notes.find(
        (n) => n.kind === args.kind && n.title === args.title.trim(),
      );
      set({ book, isLoading: false });
      return created?.id ?? null;
    } catch (error) {
      set({
        isLoading: false,
        loadError: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  },
  updateNote: async (args) => {
    set({ loadError: null });
    try {
      const book = await updateNoteCmd(args);
      set({ book });
    } catch (error) {
      set({
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
  togglePinned: async (id) => {
    try {
      const book = await togglePinnedCmd(id);
      set({ book });
    } catch (error) {
      set({
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
  toggleArchived: async (id) => {
    try {
      const book = await toggleArchivedCmd(id);
      set({ book });
    } catch (error) {
      set({
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
  deleteNote: async (id) => {
    try {
      const book = await deleteNoteCmd(id);
      set({ book });
    } catch (error) {
      set({
        loadError: error instanceof Error ? error.message : String(error),
      });
    }
  },
}));
