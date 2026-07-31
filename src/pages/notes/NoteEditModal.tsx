import { useEffect, useState } from "react";
import { useNotesStore } from "../../stores/notesStore";
import type { Note, NoteColor, NoteKind } from "../../types/notes";
import { PixelIcon } from "../../ui/PixelIcon";

const COLOR_OPTIONS: NoteColor[] = ["default", "orange", "cyan", "gold"];

/**
 * Format an epoch-ms timestamp as a `YYYY-MM-DDTHH:mm` string in the user's
 * LOCAL timezone, suitable for an `<input type="datetime-local">` value.
 *
 * This replaces the previous `new Date(ms).toISOString().slice(0,16)`, which
 * produced a UTC string but was then re-parsed as local on save — round-tripping
 * an existing reminder through the editor silently shifted it by the UTC offset
 * (e.g. 8 hours in UTC+8). Formatting in local time keeps the displayed and
 * stored value identical across an edit round-trip.
 */
function toLocalDatetimeInput(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` +
    `T${pad(d.getHours())}:${pad(d.getMinutes())}`
  );
}

/**
 * Edit/Create modal. The same component handles both flows:
 *  - `existing === null`  → creating a new note (kind from store.newKind)
 *  - `existing !== null`  → editing an existing note
 *
 * Never appears inline in the list. On save it persists and closes; on cancel
 * a brand-new empty note is deleted (created-then-cancelled == no-op).
 */
export function NoteEditModal({ existing }: { existing: Note | null }) {
  const closeEdit = useNotesStore((state) => state.closeEdit);
  const newKind = useNotesStore((state) => state.newKind);
  const createNote = useNotesStore((state) => state.createNote);
  const updateNote = useNotesStore((state) => state.updateNote);
  const deleteNote = useNotesStore((state) => state.deleteNote);

  const isCreating = existing === null;
  const kind: NoteKind = existing ? existing.kind : newKind;

  const [title, setTitle] = useState(existing?.title ?? "");
  const [body, setBody] = useState(existing?.body ?? "");
  const [color, setColor] = useState<NoteColor>(existing?.color ?? "default");
  const [memoDueAt, setMemoDueAt] = useState<string>(
    existing?.memoDueAt ? toLocalDatetimeInput(existing.memoDueAt) : "",
  );
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    // Reset local state whenever the modal's target changes (open a different note).
    setTitle(existing?.title ?? "");
    setBody(existing?.body ?? "");
    setColor(existing?.color ?? "default");
    setMemoDueAt(existing?.memoDueAt ? toLocalDatetimeInput(existing.memoDueAt) : "");
    setSaveError(null);
  }, [existing]);

  async function handleSave() {
    if (saving) return;
    setSaving(true);
    setSaveError(null);
    // Parse the datetime-local value defensively. An empty/cleared value maps
    // to null (no reminder); a malformed value must never become NaN — that
    // would slip past the type system and corrupt sorting/filtering downstream.
    const rawMs = memoDueAt ? new Date(memoDueAt).getTime() : null;
    const dueMs = rawMs !== null && Number.isFinite(rawMs) ? rawMs : null;
    // Validate reminder time for memos: block save (with a message) rather than
    // silently discarding the user's input. Past times more than 1 day ago or
    // absurd far-future values have no legitimate use and would pollute the
    // "overdue" grouping. Blocking keeps the modal open so the user can fix it.
    if (kind === "memo" && dueMs !== null) {
      const now = Date.now();
      const oneDayMs = 24 * 60 * 60 * 1000;
      const maxFutureMs = now + 80 * 365 * oneDayMs;
      if (dueMs < now - oneDayMs) {
        setSaveError("提醒时间过早，请选择更近的时间。");
        setSaving(false);
        return;
      }
      if (dueMs > maxFutureMs) {
        setSaveError("提醒时间过远，请选择更近的时间。");
        setSaving(false);
        return;
      }
    }
    try {
      if (isCreating) {
        await createNote({
          kind,
          title,
          body,
          memoDueAt: kind === "memo" ? dueMs : null,
          color,
        });
      } else if (existing) {
        await updateNote({
          id: existing.id,
          title,
          body,
          memoDueAt: kind === "memo" ? dueMs : null,
          color,
        });
      }
      closeEdit();
    } catch (error) {
      // Surface the failure instead of silently swallowing it: previously the
      // try/finally had no catch, so a failed save left the modal open with no
      // indication anything went wrong, and the user could believe it saved.
      setSaveError(
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "保存失败，请稍后重试。",
      );
    } finally {
      setSaving(false);
    }
  }

  async function handleCancel() {
    // A brand-new note that was never persisted: just close. (We only persist
    // on save, so cancel of a new note is a true no-op — nothing to delete.)
    closeEdit();
  }

  // Window-level Esc close so it works regardless of focus (the title input had
  // no Esc handler before; only the textarea did). Disabled while saving.
  useEffect(() => {
    if (saving) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        void handleCancel();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [saving]);

  const titlePlaceholder = kind === "memo" ? "备忘录标题…" : "笔记标题…";
  const bodyPlaceholder =
    kind === "memo"
      ? "写点什么…"
      : "支持 Markdown：# 标题、- 列表、**粗体**、`代码`、[链接](https://…)";
  const modalTitle = isCreating
    ? kind === "memo" ? "新建备忘录" : "新建笔记"
    : kind === "memo" ? "编辑备忘录" : "编辑笔记";

  return (
    <div className="cwp-modal-overlay" onClick={saving ? undefined : handleCancel} role="presentation">
      <div
        className={`cwp-notes-modal is-${color}`}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={modalTitle}
      >
        <div className="cwp-notes-modal-stripe" />
        <div className="cwp-notes-modal-inner">
          <div className="cwp-modal-header">
            <span className="cwp-modal-title">
              <PixelIcon name={kind === "memo" ? "lightbulb" : "log"} size={14} style={{ marginRight: "6px" }} />
              {modalTitle}
            </span>
            <button onClick={handleCancel} className="cwp-notes-modal-close" type="button" aria-label="关闭">
              ×
            </button>
          </div>

          <div className="cwp-notes-modal-body cwp-notes-modal-edit-body">
            <input
              className="cwp-note-title-input"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={titlePlaceholder}
              maxLength={80}
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void handleSave();
                }
              }}
            />

            <textarea
              className="cwp-note-editor"
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder={bodyPlaceholder}
              // Cap body length to keep the IPC payload, on-disk JSON, the
              // markdown renderer (regex per line), and the list-page plainText
              // cache bounded. 100k chars is far beyond any reasonable note
              // while preventing multi-MB pastes from freezing the editor/list.
              maxLength={100000}
              rows={kind === "memo" ? 3 : 8}
              onKeyDown={(e) => {
                // Esc is handled at the window level (see useEffect above) so it
                // works from any field; here we only keep the Ctrl/Cmd+S shortcut.
                if (e.key === "s" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  void handleSave();
                }
              }}
            />

            {kind === "memo" ? (
              <label className="cwp-note-due-row">
                <span>提醒时间（仅标记，不弹窗）</span>
                <input
                  type="datetime-local"
                  className="cwp-note-due-input"
                  value={memoDueAt}
                  onChange={(e) => setMemoDueAt(e.target.value)}
                />
              </label>
            ) : null}
          </div>

          <div className="cwp-notes-modal-foot">
            <div className="cwp-note-color-picker">
              {COLOR_OPTIONS.map((c) => (
                <button
                  key={c}
                  type="button"
                  className={`cwp-note-color-dot is-${c}${color === c ? " is-active" : ""}`}
                  onClick={() => setColor(c)}
                  title={c}
                  aria-label={`颜色 ${c}`}
                />
              ))}
            </div>
            <div className="cwp-note-edit-buttons">
              <button type="button" className="cwp-note-btn" onClick={handleCancel} disabled={saving}>
                取消
              </button>
              <button type="button" className="cwp-note-btn is-primary" onClick={handleSave} disabled={saving}>
                {saving ? "保存中…" : "保存"}
              </button>
            </div>
          </div>
          <div className="cwp-note-edit-hint">
            Ctrl+S 保存 · Esc 取消
            {saveError ? (
              <span className="cwp-note-edit-error" role="alert">
                {" "}· {saveError}
              </span>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}
