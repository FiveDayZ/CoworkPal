/**
 * Note-specific formatting helpers.
 *
 * Kept separate from `formatters.ts` (which is hardware-oriented) so the notes
 * module stays self-contained. All functions are pure and timezone-aware via
 * the platform default (the app runs local-only).
 */

/**
 * Format an epoch-ms timestamp as a short local date-time suitable for note
 * meta lines. Same-year dates use "MM-DD HH:mm"; older years include the year
 * ("YYYY-MM-DD") so cross-year notes are distinguishable at a glance.
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
  if (d.getFullYear() === new Date().getFullYear()) {
    return `${month}-${day} ${hours}:${minutes}`;
  }
  return `${d.getFullYear()}-${month}-${day}`;
}

