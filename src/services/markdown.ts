/**
 * Minimal zero-dependency Markdown renderer.
 *
 * Renders a small, safe subset of Markdown to an HTML string. The output is
 * intended for `dangerouslySetInnerHTML`; to stay safe we HTML-escape the
 * entire input FIRST, then apply Markdown transformations to the escaped
 * text, so no user-supplied HTML can ever reach the DOM.
 *
 * Supported syntax (intentionally tiny — no nesting, no tables, no images):
 *   # / ## / ###   headings
 *   - / *          unordered list items (consecutive lines become <ul>)
 *   `code`         inline code
 *   **bold**       bold
 *   [text](url)    links (url must start with http(s)://, mailto:, or #)
 *   blank line     paragraph break
 *
 * Everything else is rendered as plain (escaped) text. Lines are processed
 * top-to-bottom; this is NOT a spec-complete parser.
 */

/** HTML-escape the 5 significant characters. */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** Apply inline transformations (bold, code, links) to an already-escaped line. */
function renderInline(escaped: string): string {
  // Inline code: `...`  (do first so its content isn't re-processed)
  let out = escaped.replace(/`([^`]+)`/g, (_m, code: string) => `<code>${code}</code>`);
  // Bold: **...**
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  // Links: [text](url) — allow only safe URL schemes.
  out = out.replace(
    /\[([^\]]+)\]\(([^)\s]+)\)/g,
    (_m, text: string, url: string) => {
      if (/^(https?:\/\/|mailto:|#)/i.test(url)) {
        const target = url.startsWith("#") ? "" : ' target="_blank" rel="noopener noreferrer"';
        return `<a href="${url}"${target}>${text}</a>`;
      }
      return text; // unsafe scheme → drop the link, keep visible text
    },
  );
  return out;
}

/**
 * Render a Markdown source string to an HTML string. Returns "" for empty
 * input. Never throws.
 */
export function renderMarkdown(src: string): string {
  if (!src) {
    return "";
  }
  const escaped = escapeHtml(src);
  const lines = escaped.split(/\r?\n/);
  const html: string[] = [];

  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // Blank line → skip (paragraph spacing comes from CSS margins).
    if (trimmed === "") {
      i += 1;
      continue;
    }

    // Headings: # / ## / ###
    const headingMatch = /^(#{1,3})\s+(.*)$/.exec(trimmed);
    if (headingMatch) {
      const level = headingMatch[1].length + 2; // #→h3, ##→h4, ###→h5 (stay small)
      html.push(`<h${level}>${renderInline(headingMatch[2])}</h${level}>`);
      i += 1;
      continue;
    }

    // Unordered list: consecutive `- ` / `* ` lines → <ul>
    if (/^[-*]\s+/.test(trimmed)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*]\s+/.test(lines[i].trim())) {
        const itemText = lines[i].trim().replace(/^[-*]\s+/, "");
        items.push(`<li>${renderInline(itemText)}</li>`);
        i += 1;
      }
      html.push(`<ul>${items.join("")}</ul>`);
      continue;
    }

    // Default: paragraph (collect consecutive non-blank, non-special lines).
    const paraLines: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !/^(#{1,3})\s+/.test(lines[i].trim()) &&
      !/^[-*]\s+/.test(lines[i].trim())
    ) {
      paraLines.push(renderInline(lines[i].trim()));
      i += 1;
    }
    if (paraLines.length > 0) {
      html.push(`<p>${paraLines.join("<br/>")}</p>`);
    }
  }

  return html.join("");
}
