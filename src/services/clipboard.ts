/**
 * Reads text from the clipboard via the async Clipboard API. Returns null if
 * the API is unavailable or the user denies permission.
 */
export async function readTextFromClipboard(): Promise<string | null> {
  try {
    if (navigator.clipboard?.readText) {
      return await navigator.clipboard.readText();
    }
  } catch (error) {
    console.warn("clipboard readText failed", error);
  }
  return null;
}

/**
 * Copies text to the clipboard, preferring the async Clipboard API and falling
 * back to a hidden-textarea `execCommand("copy")` for older webviews.
 *
 * Previously duplicated verbatim in WorkLogPage and AchievementsPage.
 */
export async function copyTextToClipboard(text: string): Promise<void> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }
  } catch (error) {
    console.warn("clipboard writeText failed, falling back to execCommand", error);
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();

  try {
    document.execCommand("copy");
  } finally {
    document.body.removeChild(textarea);
  }
}
