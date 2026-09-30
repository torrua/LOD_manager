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

/** Find the best matching affix word entry (preferring Afx/Affix types and hyphenated forms over homonymous LWs). */
export function findAffixWord<T extends { name: string; type_name: string | null }>(
  words: readonly T[],
  affix: string,
  affixTypeNames: ReadonlySet<string> = new Set(['Afx', 'Affix'])
): T | undefined {
  const clean = affix.trim().replace(/^-+|-+$/g, '');
  if (!clean) return undefined;
  const cleanLower = clean.toLowerCase();

  const isAffixType = (t: string | null) => t !== null && affixTypeNames.has(t);
  const isNameMatch = (name: string) =>
    name === clean ||
    name === `${clean}-` ||
    name === `-${clean}` ||
    name.replace(/^-+|-+$/g, '') === clean ||
    name.replace(/^-+|-+$/g, '').toLowerCase() === cleanLower;

  // 1. Prefer an affix-typed word with hyphenated form (e.g. 'hei-' with type 'Afx')
  const affixHyphen = words.find(
    (w) =>
      isAffixType(w.type_name) &&
      (w.name === `${clean}-` ||
        w.name === `-${clean}` ||
        w.name.toLowerCase() === `${cleanLower}-` ||
        w.name.toLowerCase() === `-${cleanLower}`)
  );
  if (affixHyphen) return affixHyphen;

  // 2. Prefer an affix-typed word with matching name (e.g. 3/4-letter djifoa 'hum' with type 'Afx')
  const affixExact =
    words.find((w) => isAffixType(w.type_name) && w.name === clean) ??
    words.find((w) => isAffixType(w.type_name) && isNameMatch(w.name));
  if (affixExact) return affixExact;

  // 3. Fallback: hyphenated word name (e.g. 'hei-' or '-hei') regardless of type_name
  const hyphenated = words.find(
    (w) =>
      w.name === `${clean}-` ||
      w.name === `-${clean}` ||
      w.name.toLowerCase() === `${cleanLower}-` ||
      w.name.toLowerCase() === `-${cleanLower}`
  );
  if (hyphenated) return hyphenated;

  // 4. Final fallback: exact or case-insensitive name match
  return (
    words.find((w) => w.name === clean || w.name === affix) ??
    words.find((w) => w.name.toLowerCase() === cleanLower)
  );
}

/** Find a word by name, strictly preferring standalone (non-affix) exact matches before affix or hyphenated fallbacks. */
export function findWordByName<T extends { name: string; type_name?: string | null }>(
  words: readonly T[],
  name: string,
  affixTypeNames: ReadonlySet<string> = new Set(['Afx', 'Affix'])
): T | undefined {
  const trimmed = name.trim();
  if (!trimmed) return undefined;
  const isAffixType = (t: string | null | undefined) =>
    t !== null && t !== undefined && affixTypeNames.has(t);
  const lower = trimmed.toLowerCase();

  return (
    words.find((w) => w.name === trimmed && !isAffixType(w.type_name)) ??
    words.find((w) => w.name === trimmed) ??
    words.find((w) => w.name.toLowerCase() === lower && !isAffixType(w.type_name)) ??
    words.find((w) => w.name.toLowerCase() === lower) ??
    words.find((w) => w.name === `${trimmed}-` || w.name === `-${trimmed}`)
  );
}
