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

/** Apply inline transformations (bold, code, links) to an already-escaped line.
 * `query` (optional): highlights literal matches after transforms. */
function renderInline(escaped: string, query?: string): string {
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
  if (query) {
    out = highlightMatches(out, query);
  }
  return out;
}

/**
 * Escape a literal string for safe use in a RegExp (slashes, parens, etc).
 * The search query is user input matched literally — never as regex syntax.
 */
function escapeRegExp(literal: string): string {
  return literal.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Wrap case-insensitive matches of `query` in `<mark>` tags within an already-
 * HTML-escaped string. Because the input is pre-escaped (entities like &lt;
 * already resolved) and the query is matched literally against that escaped
 * text, this cannot inject HTML. Returns the input unchanged if query is empty.
 */
function highlightMatches(escaped: string, query: string): string {
  const trimmed = query.trim();
  if (!trimmed) {
    return escaped;
  }
  // Word-boundary-aware, case-insensitive, literal match.
  const pattern = new RegExp(`(${escapeRegExp(trimmed)})`, "gi");
  // Avoid highlighting inside existing tags (<code>, <a ...>, </strong>).
  return escaped
    .split(/(<[^>]+>)/)
    .map((segment) => (segment.startsWith("<") ? segment : segment.replace(pattern, "<mark>$1</mark>")))
    .join("");
}

/**
 * Render a Markdown source string to an HTML string. Returns "" for empty
 * input. Never throws.
 *
 * `query` (optional): when provided, case-insensitive matches of this literal
 * string inside the rendered text are wrapped in <mark> for search highlighting.
 */
export function renderMarkdown(src: string, query?: string): string {
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
      html.push(`<h${level}>${renderInline(headingMatch[2], query)}</h${level}>`);
      i += 1;
      continue;
    }

    // Unordered list: consecutive `- ` / `* ` lines → <ul>
    if (/^[-*]\s+/.test(trimmed)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*]\s+/.test(lines[i].trim())) {
        const itemText = lines[i].trim().replace(/^[-*]\s+/, "");
        items.push(`<li>${renderInline(itemText, query)}</li>`);
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
      paraLines.push(renderInline(lines[i].trim(), query));
      i += 1;
    }
    if (paraLines.length > 0) {
      html.push(`<p>${paraLines.join("<br/>")}</p>`);
    }
  }

  return html.join("");
}
