/**
 * Notes & memos types — mirror of the Rust `Note` family in
 * src-tauri/src/models.rs, returned by the `get_notes` / `create_note` /
 * `update_note` / `toggle_note_*` / `delete_note` commands.
 *
 * Note and Memo are a single entity distinguished by `kind`: a Note is a
 * long-form markdown record; a Memo is a short reminder-style record with an
 * optional due-time marker (display only, no alarm).
 */

export type NoteKind = "note" | "memo";

/** Label color, maps to the existing palette tone tokens. */
export type NoteColor = "default" | "orange" | "cyan" | "gold";

export interface Note {
  id: string;
  kind: NoteKind;
  title: string;
  /** Markdown source text (plain string, rendered client-side). */
  body: string;
  createdAt: number;
  updatedAt: number;
  /** Pinned notes sort to the top of the list. */
  pinned: boolean;
  /** Archived notes are soft-hidden (only visible in the 归档 filter). */
  archived: boolean;
  /** Optional due-time marker for memos (epoch ms). Display-only, no alarm. */
  memoDueAt: number | null;
  color: NoteColor;
}

export interface NoteBook {
  schemaVersion: number;
  notes: Note[];
}

/** List filter applied in the UI. */
export type NotesFilter = "all" | "note" | "memo" | "pinned" | "archived";
