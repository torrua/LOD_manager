/** Apply LOD display formatting to definition body / usage text. */
export function renderBody(text: string | null | undefined): string {
  if (!text) return '';
  let t = esc(text);
  t = t.replace(/--/g, '\u2014');
  t = t.replace(/\.\.\.\./g, '\u2026.'); // .... → ….
  t = t.replace(/\.\.\./g, '\u2026'); // ... → …
  t = t.replace(/\s%/g, ' \u2014').replace(/^%/, '\u2014');
  // «keyword» → <span class="kw">keyword</span>
  t = t.replace(/[«\u00ab]([^\u00bb»]+)[»\u00bb]/g, (_, k) => `<span class="kw">${k}</span>`);
  // {word} → clickable cross-reference
  t = t.replace(
    /\{([^}]+)\}/g,
    (_, w) => `<span class="xref" data-word="${esc(w)}">${esc(w)}</span>`
  );
  return t;
}

/** HTML-escape a string in a single pass (avoids 4 serial replace calls). */
export function esc(s: string | null | undefined): string {
  return String(s ?? '').replace(/[&<>"]/g, (c) => {
    switch (c) {
      case '&':
        return '&amp;';
      case '<':
        return '&lt;';
      case '>':
        return '&gt;';
      case '"':
        return '&quot;';
      default:
        return c;
    }
  });
}
