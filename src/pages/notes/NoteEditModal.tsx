import { useEffect, useState } from "react";
import { useNotesStore } from "../../stores/notesStore";
import type { Note, NoteColor, NoteKind } from "../../types/notes";
import { PixelIcon } from "../../ui/PixelIcon";

const COLOR_OPTIONS: NoteColor[] = ["default", "orange", "cyan", "gold"];

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
    existing?.memoDueAt ? new Date(existing.memoDueAt).toISOString().slice(0, 16) : "",
  );
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    // Reset local state whenever the modal's target changes (open a different note).
    setTitle(existing?.title ?? "");
    setBody(existing?.body ?? "");
    setColor(existing?.color ?? "default");
    setMemoDueAt(
      existing?.memoDueAt ? new Date(existing.memoDueAt).toISOString().slice(0, 16) : "",
    );
  }, [existing]);

  async function handleSave() {
    if (saving) return;
    setSaving(true);
    const dueMs = memoDueAt ? new Date(memoDueAt).getTime() : null;
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
    } finally {
      setSaving(false);
    }
  }

  async function handleCancel() {
    // A brand-new note that was never persisted: just close. (We only persist
    // on save, so cancel of a new note is a true no-op — nothing to delete.)
    closeEdit();
  }

  const titlePlaceholder = kind === "memo" ? "备忘录标题…" : "笔记标题…";
  const bodyPlaceholder =
    kind === "memo"
      ? "写点什么…"
      : "支持 Markdown：# 标题、- 列表、**粗体**、`代码`、[链接](https://…)";
  const modalTitle = isCreating
    ? kind === "memo" ? "新建备忘录" : "新建笔记"
    : kind === "memo" ? "编辑备忘录" : "编辑笔记";

  return (
    <div className="cwp-modal-overlay" onClick={handleCancel} role="presentation">
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
              rows={kind === "memo" ? 3 : 8}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  void handleCancel();
                }
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
          <div className="cwp-note-edit-hint">Ctrl+S 保存 · Esc 取消</div>
        </div>
      </div>
    </div>
  );
}
