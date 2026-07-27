import { useEffect, useMemo } from "react";
import { formatNoteTimestamp } from "../../services/noteFormatters";
import { renderMarkdown } from "../../services/markdown";
import { useNotesStore } from "../../stores/notesStore";
import type { Note, NoteKind, NotesFilter } from "../../types/notes";
import { PixelIcon } from "../../ui/PixelIcon";
import { NoteViewModal } from "./NoteViewModal";
import { NoteEditModal } from "./NoteEditModal";

const FILTER_OPTIONS: { key: NotesFilter; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "note", label: "笔记" },
  { key: "memo", label: "备忘录" },
  { key: "pinned", label: "置顶" },
  { key: "archived", label: "归档" },
];

/** Sort: pinned first, then newest updated first. */
function sortNotes(notes: Note[]): Note[] {
  return [...notes].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    return b.updatedAt - a.updatedAt;
  });
}

/** Apply the active filter to the full note list. */
function filterNotes(notes: Note[], filter: NotesFilter): Note[] {
  switch (filter) {
    case "note":
      return notes.filter((n) => n.kind === "note" && !n.archived);
    case "memo":
      return notes.filter((n) => n.kind === "memo" && !n.archived);
    case "pinned":
      return notes.filter((n) => n.pinned && !n.archived);
    case "archived":
      return notes.filter((n) => n.archived);
    case "all":
    default:
      return notes.filter((n) => !n.archived);
  }
}

export function NotesPage() {
  const book = useNotesStore((state) => state.book);
  const filter = useNotesStore((state) => state.filter);
  const isLoading = useNotesStore((state) => state.isLoading);
  const loadError = useNotesStore((state) => state.loadError);
  const loadNotes = useNotesStore((state) => state.loadNotes);
  const openEditNew = useNotesStore((state) => state.openEditNew);
  const viewId = useNotesStore((state) => state.viewId);
  const editTarget = useNotesStore((state) => state.editTarget);

  useEffect(() => {
    void loadNotes();
  }, [loadNotes]);

  const visibleNotes = useMemo(() => {
    const all = book?.notes ?? [];
    return sortNotes(filterNotes(all, filter));
  }, [book, filter]);

  function handleCreate(kind: NoteKind) {
    openEditNew(kind);
  }

  // Resolve the note currently open in the view modal (may be undefined if it
  // was deleted/archived out from under the modal).
  const viewedNote = viewId ? book?.notes.find((n) => n.id === viewId) ?? null : null;
  // Resolve the note currently open in the edit modal (null when creating new).
  const editingNote =
    editTarget && editTarget !== "new"
      ? book?.notes.find((n) => n.id === editTarget) ?? null
      : null;

  return (
    <div className="cwp-page">
      <div className="page-title-row cwp-notes-title-row">
        <h2 className="page-title">笔记与备忘录</h2>
        <div className="cwp-notes-actions">
          <button
            type="button"
            className="cwp-notes-new-btn"
            onClick={() => handleCreate("memo")}
            title="新建备忘录"
          >
            <PixelIcon name="lightbulb" size={12} /> 备忘
          </button>
          <button
            type="button"
            className="cwp-notes-new-btn is-primary"
            onClick={() => handleCreate("note")}
            title="新建笔记"
          >
            <PixelIcon name="log" size={12} /> 笔记
          </button>
        </div>
      </div>

      <div className="cwp-notes-filter-tabs">
        {FILTER_OPTIONS.map((opt) => (
          <button
            key={opt.key}
            type="button"
            className={`cwp-notes-filter-tab${filter === opt.key ? " is-active" : ""}`}
            onClick={() => useNotesStore.getState().setFilter(opt.key)}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {loadError ? (
        <div className="cwp-notes-empty">加载失败：{loadError}</div>
      ) : isLoading && !book ? (
        <div className="cwp-notes-empty">正在读取笔记…</div>
      ) : visibleNotes.length === 0 ? (
        <div className="cwp-notes-empty">
          {filter === "archived"
            ? "归档是空的。归档的笔记会出现在这里。"
            : "还没有任何记录。点击右上角「笔记」或「备忘」开始记录吧。"}
        </div>
      ) : (
        <div className="cwp-notes-content">
          {visibleNotes.map((note) => (
            <NoteListItem key={note.id} note={note} />
          ))}
        </div>
      )}

      {/* View modal — read-only, opened by clicking a list item. */}
      {viewedNote ? <NoteViewModal note={viewedNote} /> : null}
      {/* Edit modal — for both new and existing notes. */}
      {editTarget ? <NoteEditModal existing={editingNote} /> : null}
    </div>
  );
}

/** A read-only list row. Clicking opens the view modal (never edits inline). */
function NoteListItem({ note }: { note: Note }) {
  const openView = useNotesStore((state) => state.openView);
  const bodyHtml = useMemo(() => renderMarkdown(note.body), [note.body]);

  return (
    <article
      className={`cwp-note-card is-${note.color} is-${note.kind}`}
      onClick={() => openView(note.id)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          openView(note.id);
        }
      }}
    >
      <div className="cwp-note-card-stripe" />
      <div className="cwp-note-card-body">
        <div className="cwp-note-card-head">
          <span className="cwp-note-kind-tag">{note.kind === "memo" ? "备忘" : "笔记"}</span>
          {note.title ? (
            <strong className="cwp-note-title">{note.title}</strong>
          ) : (
            <strong className="cwp-note-title is-empty">无标题</strong>
          )}
          {note.pinned ? <span className="cwp-note-pin" title="已置顶">★</span> : null}
        </div>

        {note.body ? (
          <div
            className="cwp-note-preview"
            dangerouslySetInnerHTML={{ __html: bodyHtml }}
          />
        ) : (
          <div className="cwp-note-preview is-empty">空内容</div>
        )}

        <div className="cwp-note-meta">
          <span>{formatNoteTimestamp(note.updatedAt)}</span>
          {note.memoDueAt ? (
            <span className="cwp-note-due">⏰ {formatNoteTimestamp(note.memoDueAt)}</span>
          ) : null}
        </div>
      </div>
    </article>
  );
}
