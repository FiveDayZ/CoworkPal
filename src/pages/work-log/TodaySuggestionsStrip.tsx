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
  const loadTodaySuggestions = useSuggestionsStore(
    (state) => state.loadTodaySuggestions,
  );
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Load once on mount. Suggestions are day-scoped, so a single fetch per visit
  // is enough; the user can re-enter the dashboard to refresh.
  useEffect(() => {
    void loadTodaySuggestions();
  }, [loadTodaySuggestions]);

  const suggestions = today?.top ?? [];

  if (suggestions.length === 0) {
    // Don't render an empty block — keep the dashboard clean when there's no
    // advice yet (e.g. fresh install with < 3 days of history).
    if (!isLoading) {
      return null;
    }
    return (
      <section className="cwp-suggestions-strip">
        <div className="cwp-section-title">
          <PixelIcon name="lightbulb" size={14} />
          <strong>今日建议</strong>
        </div>
        <div className="cwp-suggestions-empty">CoCat 正在分析最近的使用习惯…</div>
      </section>
    );
  }

  return (
    <section className="cwp-suggestions-strip">
      <div className="cwp-section-title">
        <PixelIcon name="lightbulb" size={14} />
        <strong>今日建议</strong>
        <span className="cwp-suggestions-count">{suggestions.length} 条</span>
      </div>
      <div className="cwp-suggestions-grid">
        {suggestions.map((suggestion) => (
          <SuggestionCard
            key={suggestion.id}
            suggestion={suggestion}
            expanded={expandedId === suggestion.id}
            onToggle={() =>
              setExpandedId((current) =>
                current === suggestion.id ? null : suggestion.id,
              )
            }
          />
        ))}
      </div>
    </section>
  );
}

function SuggestionCard({
  suggestion,
  expanded,
  onToggle,
}: {
  suggestion: Suggestion;
  expanded: boolean;
  onToggle: () => void;
}) {
  const severityClass = `is-${suggestion.severity}`;
  const iconName = SUGGESTION_ICON[suggestion.category] ?? "lightbulb";
  return (
    <article
      className={`cwp-suggestion-card ${severityClass}`}
      onClick={onToggle}
      role="button"
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onToggle();
        }
      }}
    >
      <div className="cwp-suggestion-head">
        <span className="cwp-suggestion-icon">
          <PixelIcon name={iconName} size={14} />
        </span>
        <strong className="cwp-suggestion-title">{suggestion.title}</strong>
      </div>
      <p className="cwp-suggestion-body">{suggestion.body}</p>
      {expanded ? (
        <p className="cwp-suggestion-action">{suggestion.actionHint}</p>
      ) : null}
    </article>
  );
}
