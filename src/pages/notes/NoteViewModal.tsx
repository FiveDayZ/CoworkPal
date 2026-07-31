import { useEffect, useMemo, useRef, useState } from "react";
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
 * Supports an immersive fullscreen reading mode with day/night themes:
 *  - normal: the default themed look (inherits app colors)
 *  - day:     light paper-like background (#faf8f5) with dark text — easy on
 *             the eyes in bright environments
 *  - night:   dark background (#2d2d2d) with soft light text (#e0e0e0) —
 *             reduces eye strain and blue light in low-light environments
 */
export function NoteViewModal({ note }: { note: Note }) {
  const closeView = useNotesStore((state) => state.closeView);
  const openEditExisting = useNotesStore((state) => state.openEditExisting);
  const togglePinned = useNotesStore((state) => state.togglePinned);
  const toggleArchived = useNotesStore((state) => state.toggleArchived);
  const deleteNote = useNotesStore((state) => state.deleteNote);
  const exportNote = useNotesStore((state) => state.exportNote);

  // `busy` gates ALL action buttons while any mutation is in flight, so a user
  // cannot double-click delete/​toggle and fire duplicate IPC calls (which the
  // store would then race). `toast` surfaces success/failure inline instead of
  // the previous blocking window.alert / silent failure.
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  // Track the toast auto-hide timer so consecutive flashes replace (not stack)
  // — otherwise an earlier timer would clear a later toast prematurely.
  const toastTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Immersive fullscreen reading + day/night theme. `readingMode` cycles
  // normal → day → night → normal; `fullscreen` toggles the distraction-free
  // layout (full window, hidden action bar, large centered text).
  const [fullscreen, setFullscreen] = useState(false);
  const [readingMode, setReadingMode] = useState<"normal" | "day" | "night">("normal");

  const bodyHtml = useMemo(() => renderMarkdown(note.body), [note.body]);

  // Esc closes the view modal (matching NoteEditModal's behavior), unless an
  // action is in flight (busy) — closing mid-delete would hide the result.
  // In fullscreen, the first Esc exits fullscreen instead of closing the modal.
  useEffect(() => {
    if (busy) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        if (fullscreen) {
          setFullscreen(false);
        } else {
          closeView();
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, closeView, fullscreen]);

  // Clear any pending toast timer on unmount to avoid setState after unmount.
  useEffect(() => {
    return () => {
      if (toastTimerRef.current) {
        clearTimeout(toastTimerRef.current);
      }
    };
  }, []);

  function flashToast(message: string) {
    setToast(message);
    if (toastTimerRef.current) {
      clearTimeout(toastTimerRef.current);
    }
    toastTimerRef.current = setTimeout(() => {
      toastTimerRef.current = null;
      setToast(null);
    }, 2600);
  }

  async function handleExport() {
    if (busy) return;
    setBusy(true);
    try {
      const path = await exportNote(note.id);
      if (path) {
        flashToast(`已导出到：${path}`);
      }
    } catch (error) {
      flashToast(
        `导出失败：${error instanceof Error ? error.message : String(error)}`,
      );
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (busy) return;
    if (!window.confirm("确定删除这条记录吗？此操作不可撤销。")) {
      return;
    }
    setBusy(true);
    try {
      // Await the delete BEFORE closing: if it fails the modal stays open and
      // surfaces the error, rather than closing and hiding the failure from the
      // user (who would believe the note was deleted while it still exists).
      await deleteNote(note.id);
      closeView();
    } catch (error) {
      setBusy(false);
      flashToast(
        `删除失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async function handleTogglePinned() {
    if (busy) return;
    setBusy(true);
    try {
      await togglePinned(note.id);
    } catch (error) {
      flashToast(
        `操作失败：${error instanceof Error ? error.message : String(error)}`,
      );
    } finally {
      setBusy(false);
    }
  }

  async function handleToggleArchived() {
    if (busy) return;
    setBusy(true);
    try {
      await toggleArchived(note.id);
      closeView();
    } catch (error) {
      setBusy(false);
      flashToast(
        `操作失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  return (
    <div
      className={`cwp-modal-overlay${fullscreen ? " is-fullscreen" : ""}`}
      onClick={busy ? undefined : closeView}
      role="presentation"
    >
      <div
        className={`cwp-notes-modal is-${note.color}${fullscreen ? " is-fullscreen" : ""} is-reading-${readingMode}`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={note.title || "笔记详情"}
      >
        <div className="cwp-notes-modal-stripe" />
        <div className="cwp-notes-modal-inner">
          {/* Header: kind tag + title + reading controls + close */}
          <div className="cwp-modal-header">
            <span className="cwp-modal-title">
              <PixelIcon name={note.kind === "memo" ? "lightbulb" : "log"} size={14} style={{ marginRight: "6px" }} />
              {note.title || (note.kind === "memo" ? "无标题备忘录" : "无标题笔记")}
            </span>
            <div className="cwp-notes-modal-header-actions">
              {/* Reading-mode cycle: normal → day → night → normal */}
              <button
                type="button"
                className="cwp-note-reading-toggle"
                onClick={() =>
                  setReadingMode((m) => (m === "normal" ? "day" : m === "day" ? "night" : "normal"))
                }
                title={
                  readingMode === "normal"
                    ? "切换到白天阅读模式"
                    : readingMode === "day"
                      ? "切换到夜间阅读模式"
                      : "切换到默认模式"
                }
                aria-label="切换阅读模式"
              >
                {readingMode === "normal" ? "☀" : readingMode === "day" ? "🌙" : "◐"}
              </button>
              {/* Fullscreen toggle */}
              <button
                type="button"
                className="cwp-note-reading-toggle"
                onClick={() => setFullscreen((f) => !f)}
                title={fullscreen ? "退出全屏阅读" : "全屏阅读"}
                aria-label="全屏阅读"
              >
                {fullscreen ? "⤢" : "⛶"}
              </button>
              <button onClick={closeView} className="cwp-notes-modal-close" type="button" aria-label="关闭" disabled={busy}>
                ×
              </button>
            </div>
          </div>

          {/* Meta: timestamps (hidden in fullscreen for distraction-free reading) */}
          {!fullscreen ? (
            <div className="cwp-notes-modal-meta">
              <span>创建 {formatNoteTimestamp(note.createdAt)}</span>
              <span>更新 {formatNoteTimestamp(note.updatedAt)}</span>
              {note.memoDueAt ? (
                <span className="cwp-note-due">⏰ {formatNoteTimestamp(note.memoDueAt)}</span>
              ) : null}
              {note.pinned ? <span className="cwp-note-pin">★ 已置顶</span> : null}
            </div>
          ) : null}

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

          {/* Inline feedback for export/delete/toggle outcomes. */}
          {toast ? <div className="cwp-note-view-toast" role="status">{toast}</div> : null}

          {/* Footer actions (hidden in fullscreen for immersive reading) */}
          {!fullscreen ? (
            <div className="cwp-notes-modal-actions">
              <button
                type="button"
                className="cwp-note-btn is-primary"
                onClick={() => openEditExisting(note.id)}
                disabled={busy}
              >
                编辑
              </button>
              <button
                type="button"
                className="cwp-note-btn"
                onClick={() => void handleExport()}
                title="导出为 .md 文件"
                disabled={busy}
              >
                {busy ? "…" : "导出"}
              </button>
              <button
                type="button"
                className="cwp-note-btn"
                onClick={() => void handleTogglePinned()}
                disabled={busy}
              >
                {note.pinned ? "取消置顶" : "置顶"}
              </button>
              <button
                type="button"
                className="cwp-note-btn"
                onClick={() => void handleToggleArchived()}
                disabled={busy}
              >
                {note.archived ? "取消归档" : "归档"}
              </button>
              <button
                type="button"
                className="cwp-note-btn is-danger"
                onClick={() => void handleDelete()}
                disabled={busy}
              >
                {busy ? "…" : "删除"}
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
