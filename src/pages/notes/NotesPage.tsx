import { useEffect, useMemo, useRef, useState } from "react";
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

/** Count notes per filter tab (independent of the active filter/search). */
function countByFilter(notes: Note[]): Record<NotesFilter, number> {
  let note = 0,
    memo = 0,
    pinned = 0,
    archived = 0;
  for (const n of notes) {
    if (n.archived) {
      archived += 1;
      continue;
    }
    if (n.kind === "note") note += 1;
    else memo += 1;
    if (n.pinned) pinned += 1;
  }
  return {
    all: note + memo,
    note,
    memo,
    pinned,
    archived,
  };
}

/** Strip markdown syntax for plain-text search matching.
 *  Results are memoized by the body string so repeated keystrokes don't
 *  re-run the regex over the same note body every time. */
const plainTextCache = new Map<string, string>();
const PLAIN_TEXT_CACHE_LIMIT = 500;

function plainText(body: string): string {
  const cached = plainTextCache.get(body);
  if (cached !== undefined) {
    return cached;
  }
  const result = body
    .replace(/[`*_#>-]/g, " ")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .toLowerCase();
  // Bounded FIFO eviction (NOT LRU — a cache hit doesn't move the entry to the
  // back): when full, drop the oldest-inserted key. Sufficient for the search
  // use case since misses are cheap. Body length is capped at 100k by the
  // editor (NoteEditModal maxLength), so 500 entries bound worst-case memory.
  if (plainTextCache.size >= PLAIN_TEXT_CACHE_LIMIT) {
    plainTextCache.delete(plainTextCache.keys().next().value as string);
  }
  plainTextCache.set(body, result);
  return result;
}

/** Case-insensitive substring match on title + plain-text body. */
function matchesQuery(note: Note, query: string): boolean {
  if (!query) return true;
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return note.title.toLowerCase().includes(q) || plainText(note.body).includes(q);
}

/** Memo due-time quick filter presets. */
type DueFilter = "any" | "today" | "week" | "overdue" | "upcoming";

const DUE_OPTIONS: { key: DueFilter; label: string }[] = [
  { key: "any", label: "不限" },
  { key: "today", label: "今天" },
  { key: "week", label: "本周" },
  { key: "overdue", label: "已过期" },
  { key: "upcoming", label: "即将到期" },
];

/** Convert a YYYY-MM-DD string to epoch-ms (start of that local day). */
function dayStart(dateStr: string): number | null {
  if (!dateStr) return null;
  const d = new Date(`${dateStr}T00:00:00`);
  return Number.isNaN(d.getTime()) ? null : d.getTime();
}

/** Whether a note's createdAt falls within [from, to] (inclusive day range). */
function inDateRange(note: Note, fromMs: number | null, toMs: number | null): boolean {
  if (fromMs === null && toMs === null) return true;
  const created = note.createdAt;
  if (fromMs !== null && created < fromMs) return false;
  // toMs is start-of-day; include the whole day → add 24h.
  if (toMs !== null && created > toMs + 24 * 60 * 60 * 1000 - 1) return false;
  return true;
}

/** Whether a memo's due time matches the preset. Notes (no due) only match "any". */
function matchesDue(note: Note, due: DueFilter): boolean {
  if (due === "any") return true;
  if (!note.memoDueAt) return false;
  const now = Date.now();
  const startToday = new Date();
  startToday.setHours(0, 0, 0, 0);
  const endToday = startToday.getTime() + 24 * 60 * 60 * 1000;
  const endOfWeek = endToday + 6 * 24 * 60 * 60 * 1000;
  const dueMs = note.memoDueAt;
  switch (due) {
    case "today":
      return dueMs >= startToday.getTime() && dueMs < endToday;
    case "week":
      return dueMs >= startToday.getTime() && dueMs < endOfWeek;
    case "overdue":
      return dueMs < startToday.getTime();
    case "upcoming":
      return dueMs >= now && dueMs < now + 7 * 24 * 60 * 60 * 1000;
    default:
      return true;
  }
}

export function NotesPage() {
  const book = useNotesStore((state) => state.book);
  const filter = useNotesStore((state) => state.filter);
  const isLoading = useNotesStore((state) => state.isLoading);
  const loadError = useNotesStore((state) => state.loadError);
  const loadNotes = useNotesStore((state) => state.loadNotes);
  const openEditNew = useNotesStore((state) => state.openEditNew);
  const importNote = useNotesStore((state) => state.importNote);
  const viewId = useNotesStore((state) => state.viewId);
  const editTarget = useNotesStore((state) => state.editTarget);

  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [dueFilter, setDueFilter] = useState<DueFilter>("any");
  const searchRef = useRef<HTMLInputElement>(null);

  // Debounce the search query so rapid typing doesn't scan all notes on every
  // keystroke. 120ms is imperceptible to humans but coalesces fast input.
  useEffect(() => {
    const handle = window.setTimeout(() => setDebouncedQuery(query), 120);
    return () => window.clearTimeout(handle);
  }, [query]);

  useEffect(() => {
    void loadNotes();
  }, [loadNotes]);

  // "/" focuses the search box; Esc (when search already focused) clears it.
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const typing = target.tagName === "INPUT" || target.tagName === "TEXTAREA";
      if (e.key === "/" && !typing) {
        e.preventDefault();
        searchRef.current?.focus();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const allNotes = book?.notes ?? [];
  const counts = useMemo(() => countByFilter(allNotes), [allNotes]);

  const fromMs = useMemo(() => dayStart(dateFrom), [dateFrom]);
  const toMs = useMemo(() => dayStart(dateTo), [dateTo]);
  const hasDateFilter = fromMs !== null || toMs !== null;
  const hasDueFilter = dueFilter !== "any";
  const hasAdvancedFilter = hasDateFilter || hasDueFilter;

  // Search filtering AND highlighting both use the debounced query. Highlight
  // used to use the live query, but that re-ran renderMarkdown (regex per line)
  // for every note on every keystroke — a CPU spike with many/long notes. The
  // 120ms debounce is imperceptible to the user yet bounds the work.
  const visibleNotes = useMemo(() => {
    const filtered = filterNotes(allNotes, filter);
    const searched = debouncedQuery.trim()
      ? filtered.filter((n) => matchesQuery(n, debouncedQuery))
      : filtered;
    const byDate = hasDateFilter
      ? searched.filter((n) => inDateRange(n, fromMs, toMs))
      : searched;
    const byDue = hasDueFilter ? byDate.filter((n) => matchesDue(n, dueFilter)) : byDate;
    return sortNotes(byDue);
  }, [allNotes, filter, debouncedQuery, hasDateFilter, fromMs, toMs, hasDueFilter, dueFilter]);

  function clearAdvancedFilters() {
    setDateFrom("");
    setDateTo("");
    setDueFilter("any");
  }

  function handleCreate(kind: NoteKind) {
    openEditNew(kind);
  }

  // Import a .md file as a new note. The backend shows the open dialog and
  // reads the file; on success the store reloads and the new note appears.
  const [importing, setImporting] = useState(false);
  async function handleImport() {
    if (importing) return;
    setImporting(true);
    try {
      await importNote();
    } catch (error) {
      console.error("failed to import note", error);
    } finally {
      setImporting(false);
    }
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
    <div className="cwp-page cwp-notes-page">
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
          <button
            type="button"
            className="cwp-notes-new-btn"
            onClick={() => void handleImport()}
            disabled={importing}
            title="从 Markdown 文件导入"
          >
            <PixelIcon name="log" size={12} /> {importing ? "导入中…" : "导入"}
          </button>
        </div>
      </div>

      <div className="cwp-notes-body">
        <div className="cwp-notes-toolbar">
          <div className="cwp-notes-filter-tabs">
            {FILTER_OPTIONS.map((opt) => (
              <button
                key={opt.key}
                type="button"
                className={`cwp-notes-filter-tab${filter === opt.key ? " is-active" : ""}`}
                onClick={() => useNotesStore.getState().setFilter(opt.key)}
              >
                {opt.label}
                <span className="cwp-notes-tab-count">{counts[opt.key]}</span>
              </button>
            ))}
          </div>
          <div className="cwp-notes-search-row">
            <PixelIcon name="puzzle" size={12} style={{ color: "var(--color-text-muted)", flex: "0 0 12px" }} />
            <input
              ref={searchRef}
              className="cwp-notes-search-input"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  setQuery("");
                  searchRef.current?.blur();
                }
              }}
              placeholder="搜索标题或正文…"
              type="text"
            />
            {query ? (
              <button
                type="button"
                className="cwp-notes-search-clear"
                onClick={() => setQuery("")}
                aria-label="清除搜索"
              >
                ×
              </button>
            ) : null}
          </div>
        </div>

        {/* Advanced filters: date range + memo due preset. */}
        <div className="cwp-notes-advanced">
          <div className="cwp-notes-date-range">
            <PixelIcon name="calendar" size={12} style={{ color: "var(--color-text-muted)" }} />
            <input
              type="date"
              className="cwp-notes-date-input"
              value={dateFrom}
              onChange={(e) => setDateFrom(e.target.value)}
              max={dateTo || undefined}
              aria-label="起始日期"
            />
            <span className="cwp-notes-date-sep">~</span>
            <input
              type="date"
              className="cwp-notes-date-input"
              value={dateTo}
              onChange={(e) => setDateTo(e.target.value)}
              min={dateFrom || undefined}
              aria-label="结束日期"
            />
          </div>
          <div className="cwp-notes-due-tabs">
            {DUE_OPTIONS.map((opt) => (
              <button
                key={opt.key}
                type="button"
                className={`cwp-notes-due-tab${dueFilter === opt.key ? " is-active" : ""}`}
                onClick={() => setDueFilter(opt.key)}
                title={opt.key === "any" ? "不限到期时间" : "按备忘录提醒时间筛选"}
              >
                {opt.label}
              </button>
            ))}
          </div>
          {hasAdvancedFilter ? (
            <button
              type="button"
              className="cwp-notes-filter-clear"
              onClick={clearAdvancedFilters}
            >
              清除筛选
            </button>
          ) : null}
        </div>

        {loadError ? (
          <div className="cwp-notes-empty">加载失败：{loadError}</div>
        ) : isLoading && !book ? (
          <div className="cwp-notes-empty">正在读取笔记…</div>
        ) : visibleNotes.length === 0 ? (
          <div className="cwp-notes-empty">
            {debouncedQuery.trim()
              ? `没有匹配「${debouncedQuery.trim()}」的记录。试试换个关键词或切换筛选。`
              : filter === "archived"
                ? "归档是空的。归档的笔记会出现在这里。"
                : allNotes.length === 0
                  ? "还没有任何记录。点击右上角「笔记」或「备忘」开始记录吧。"
                  : "当前筛选下没有记录，试试切换上方的分类标签。"}
          </div>
        ) : (
          <div className="cwp-notes-content">
            {visibleNotes.map((note) => (
              <NoteListItem key={note.id} note={note} query={debouncedQuery} />
            ))}
          </div>
        )}
      </div>

      {/* View modal — read-only, opened by clicking a list item. */}
      {viewedNote ? <NoteViewModal note={viewedNote} /> : null}
      {/* Edit modal — for both new and existing notes. */}
      {editTarget ? <NoteEditModal existing={editingNote} /> : null}
    </div>
  );
}

/** Escape HTML and highlight query matches in a plain-text string. */
function highlightPlainText(text: string, query: string): { __html: string } {
  const escaped = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  const q = query.trim();
  if (!q) {
    return { __html: escaped };
  }
  const pattern = new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");
  return { __html: escaped.replace(pattern, "<mark>$1</mark>") };
}

/** A read-only list row. Clicking opens the view modal (never edits inline). */
function NoteListItem({ note, query }: { note: Note; query: string }) {
  const openView = useNotesStore((state) => state.openView);
  const bodyHtml = useMemo(() => renderMarkdown(note.body, query), [note.body, query]);
  const titleHtml = useMemo(
    () => highlightPlainText(note.title || (note.kind === "memo" ? "无标题备忘录" : "无标题笔记"), query),
    [note.title, note.kind, query],
  );

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
          <strong
            className={`cwp-note-title${note.title ? "" : " is-empty"}`}
            dangerouslySetInnerHTML={titleHtml}
          />
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
