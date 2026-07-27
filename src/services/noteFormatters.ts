/**
 * Note-specific formatting helpers.
 *
 * Kept separate from `formatters.ts` (which is hardware-oriented) so the notes
 * module stays self-contained. All functions are pure and timezone-aware via
 * the platform default (the app runs local-only).
 */

/**
 * Format an epoch-ms timestamp as a short local date-time suitable for note
 * meta lines, e.g. "07-27 14:30" for today or "06-15 09:00" for an older date.
 * Returns "—" for falsy values.
 */
export function formatNoteTimestamp(timestampMs: number | null | undefined): string {
  if (!timestampMs) {
    return "—";
  }
  const d = new Date(timestampMs);
  const pad = (n: number) => String(n).padStart(2, "0");
  const month = pad(d.getMonth() + 1);
  const day = pad(d.getDate());
  const hours = pad(d.getHours());
  const minutes = pad(d.getMinutes());
  return `${month}-${day} ${hours}:${minutes}`;
}
