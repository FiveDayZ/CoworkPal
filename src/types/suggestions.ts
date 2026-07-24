/**
 * Smart-suggestion types — mirror of the Rust `Suggestion` family in
 * src-tauri/src/models.rs, returned by the `get_today_suggestions` command.
 */

export type SuggestionCategory =
  | "focusHabit"
  | "thermal"
  | "memory"
  | "rhythm"
  | "workload"
  | "streak";

export type SuggestionSeverity = "positive" | "neutral" | "warning";

export interface Suggestion {
  id: string;
  category: SuggestionCategory;
  priority: number;
  title: string;
  body: string;
  actionHint: string;
  severity: SuggestionSeverity;
}

export interface TodaySuggestions {
  date: string;
  top: Suggestion[];
  all: Suggestion[];
  generatedAt: number;
}
