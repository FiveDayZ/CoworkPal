import { useUiStore } from "../../stores/uiStore";
import { readDismissedSuggestions, dismissSuggestion } from "../../services/suggestionFeedback";
import { useEffect, useState } from "react";
import { useSuggestionsStore } from "../../stores/suggestionsStore";
import type { Suggestion, SuggestionCategory } from "../../types/suggestions";
import { PixelIcon, type PixelIconName } from "../../ui/PixelIcon";

/** Icon per suggestion category. */
const SUGGESTION_ICON: Record<SuggestionCategory, PixelIconName> = {
  focusHabit: "focus",
  thermal: "temp",
  memory: "ram",
  rhythm: "calendar",
  workload: "cpu",
  streak: "sparkle",
};

/**
 * Compact "today's suggestions" strip for the Dashboard. Shows the top-priority
 * advice from the local suggestion engine (history + current snapshot).
 *
 * Collapsible: click a card to expand the action hint. Falls back to a small
 * hint when there are no suggestions (e.g. not enough history yet).
 */
export function TodaySuggestionsStrip() {
  const today = useSuggestionsStore((state) => state.today);
  const isLoading = useSuggestionsStore((state) => state.isLoading);
  const loadError = useSuggestionsStore((state) => state.loadError);
  const load = useSuggestionsStore((state) => state.loadTodaySuggestions);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState<string[]>([]);
  const [feedbackError, setFeedbackError] = useState<string | null>(null);
  useEffect(() => { void load(); }, [load]);
  useEffect(() => {
    if (!today) return;
    try { setDismissed(readDismissedSuggestions(localStorage, today.date)); }
    catch { setFeedbackError("无法读取建议偏好，当前仍显示全部建议。"); }
  }, [today?.date]);
  function dismiss(id: string) {
    if (!today) return;
    try { setDismissed(dismissSuggestion(localStorage, today.date, id)); setFeedbackError(null); }
    catch { setFeedbackError("忽略偏好未保存，请检查本地存储后重试。"); }
  }
  const suggestions = (today?.all ?? []).filter((item) => !dismissed.includes(item.id)).slice(0, 3);
  return <section className="cwp-suggestions-strip">
    <div className="cwp-section-title"><PixelIcon name="lightbulb" size={14} /><strong>今日建议</strong></div>
    {loadError ? <p role="alert">加载失败：{loadError} <button type="button" className="cwp-compact-button" onClick={() => void load()}>重试</button></p> : null}
    {feedbackError ? <p role="alert">{feedbackError}</p> : null}
    <div className="cwp-suggestions-grid">{suggestions.map((suggestion) => <SuggestionCard key={suggestion.id} suggestion={suggestion} expanded={expandedId === suggestion.id} onToggle={() => setExpandedId(expandedId === suggestion.id ? null : suggestion.id)} onDismiss={() => dismiss(suggestion.id)} />)}</div>
    {!suggestions.length && !loadError ? <p className="cwp-suggestions-empty">{isLoading ? "正在分析最近的使用习惯…" : dismissed.length ? "今天的建议已处理，明天会重新评估。" : "暂时没有需要处理的建议，积累使用记录后再来看看。"}</p> : null}
  </section>;
}
function SuggestionCard({ suggestion, expanded, onToggle, onDismiss }: { suggestion: Suggestion; expanded: boolean; onToggle: () => void; onDismiss: () => void }) {
  const focus = suggestion.category === "focusHabit" || suggestion.category === "streak";
  const performance = ["thermal", "memory", "workload"].includes(suggestion.category);
  function act() {
    const ui = useUiStore.getState();
    ui.setMainRoute(focus ? "focus" : "workshop");
    if (!focus) ui.setWorkshopTab("review");
    if (performance) ui.setPerformanceHistoryOpen(true);
  }
  return <article className={`cwp-suggestion-card is-${suggestion.severity}`}>
    <button type="button" className="cwp-suggestion-head" aria-expanded={expanded} onClick={onToggle}>
      <span className="cwp-suggestion-icon"><PixelIcon name={SUGGESTION_ICON[suggestion.category] ?? "lightbulb"} size={14} /></span><strong className="cwp-suggestion-title">{suggestion.title}</strong>
    </button>
    <p className="cwp-suggestion-body">{suggestion.body}</p>
    {expanded ? <><p className="cwp-suggestion-action">{suggestion.actionHint}</p><div className="cwp-inline-actions"><button type="button" className="cwp-compact-button" onClick={act}>{focus ? "开始专注" : performance ? "查看性能事件" : "调整陪伴场景"}</button><button type="button" className="cwp-compact-button" onClick={onDismiss}>今日忽略</button></div></> : null}
  </article>;
}
