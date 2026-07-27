import { useMemo } from "react";
import { formatNoteTimestamp } from "../../services/noteFormatters";
import { renderMarkdown } from "../../services/markdown";
import { useNotesStore } from "../../stores/notesStore";
import type { Note } from "../../types/notes";
import { PixelIcon } from "../../ui/PixelIcon";

/**
 * Read-only note view modal. Opened by clicking a list item. Shows the
 * rendered markdown body and a small action row (edit / pin / archive /
 * delete). Editing or deleting transitions out of this modal into the edit
 * modal (or back to the list).
 *
 * Styled as a centered overlay + glass panel, matching the UpdateModal /
 * workshop-detail-modal convention.
 */
export function NoteViewModal({ note }: { note: Note }) {
  const closeView = useNotesStore((state) => state.closeView);
  const openEditExisting = useNotesStore((state) => state.openEditExisting);
  const togglePinned = useNotesStore((state) => state.togglePinned);
  const toggleArchived = useNotesStore((state) => state.toggleArchived);
  const deleteNote = useNotesStore((state) => state.deleteNote);

  const bodyHtml = useMemo(() => renderMarkdown(note.body), [note.body]);

  function handleDelete() {
    if (window.confirm("确定删除这条记录吗？此操作不可撤销。")) {
      void deleteNote(note.id);
      closeView();
    }
  }

  return (
    <div
      className="cwp-modal-overlay"
      onClick={closeView}
      role="presentation"
    >
      <div
        className={`cwp-notes-modal is-${note.color}`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={note.title || "笔记详情"}
      >
        <div className="cwp-notes-modal-stripe" />
        <div className="cwp-notes-modal-inner">
          {/* Header: kind tag + title + close */}
          <div className="cwp-modal-header">
            <span className="cwp-modal-title">
              <PixelIcon name={note.kind === "memo" ? "lightbulb" : "log"} size={14} style={{ marginRight: "6px" }} />
              {note.title || (note.kind === "memo" ? "无标题备忘录" : "无标题笔记")}
            </span>
            <button onClick={closeView} className="cwp-notes-modal-close" type="button" aria-label="关闭">
              ×
            </button>
          </div>

          {/* Meta: timestamps */}
          <div className="cwp-notes-modal-meta">
            <span>创建 {formatNoteTimestamp(note.createdAt)}</span>
            <span>更新 {formatNoteTimestamp(note.updatedAt)}</span>
            {note.memoDueAt ? (
              <span className="cwp-note-due">⏰ {formatNoteTimestamp(note.memoDueAt)}</span>
            ) : null}
            {note.pinned ? <span className="cwp-note-pin">★ 已置顶</span> : null}
          </div>

          {/* Body: rendered markdown */}
          <div className="cwp-notes-modal-body">
            {note.body ? (
              <div
                className="cwp-note-rendered"
                dangerouslySetInnerHTML={{ __html: bodyHtml }}
              />
            ) : (
              <div className="cwp-note-preview is-empty">这条记录还没有内容。</div>
            )}
          </div>

          {/* Footer actions */}
          <div className="cwp-notes-modal-actions">
            <button
              type="button"
              className="cwp-note-btn is-primary"
              onClick={() => openEditExisting(note.id)}
            >
              编辑
            </button>
            <button
              type="button"
              className="cwp-note-btn"
              onClick={() => void togglePinned(note.id)}
            >
              {note.pinned ? "取消置顶" : "置顶"}
            </button>
            <button
              type="button"
              className="cwp-note-btn"
              onClick={() => {
                void toggleArchived(note.id);
                closeView();
              }}
            >
              {note.archived ? "取消归档" : "归档"}
            </button>
            <button
              type="button"
              className="cwp-note-btn is-danger"
              onClick={handleDelete}
            >
              删除
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
