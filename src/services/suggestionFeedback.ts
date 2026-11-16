const KEY = "coworkpal.suggestion-feedback";
export function readDismissedSuggestions(storage: Pick<Storage, "getItem">, date: string): string[] {
  const raw = storage.getItem(KEY);
  if (!raw) return [];
  const saved: unknown = JSON.parse(raw);
  if (!saved || typeof saved !== "object" || !("date" in saved) || !("ids" in saved)) return [];
  return saved.date === date && Array.isArray(saved.ids) ? saved.ids.filter((id): id is string => typeof id === "string").slice(0, 20) : [];
}
export function dismissSuggestion(storage: Pick<Storage, "getItem" | "setItem">, date: string, id: string): string[] {
  const ids = [...new Set([...readDismissedSuggestions(storage, date), id])].slice(-20);
  storage.setItem(KEY, JSON.stringify({ date, ids }));
  return ids;
}
