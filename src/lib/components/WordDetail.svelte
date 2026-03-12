<!-- eslint-disable svelte/no-at-html-tags -->
<script lang="ts">
  import { app, deleteWord, saveDef, deleteDef, selectWord, applyFilter } from '../store.svelte';
  import Icon from './Icon.svelte';

  // ── Tooltip lookup tables ─────────────────────────────────────────────────────
  const GRAMMAR_TIPS: Record<string, string> = {
    n: 'Noun',
    a: 'Adjective',
    av: 'Adverb',
    v: 'Verb',
    va: 'Verb auxiliary',
    c: 'Conjunction',
    p: 'Pronoun',
    pp: 'Preposition',
    i: 'Interjection',
    h: 'Honorific',
    l: 'Letter',
    ms: 'Mathematical sign',
    na: 'Name (proper noun)',
    op: 'Operator',
    ph: 'Phrase',
    pm: 'Punctuation mark',
    voc: 'Vocative',
    af: 'Affix (combining form)',
    art: 'Article',
  };
  function grammarTip(code: string): string {
    const m = code.match(/^(\d+)(.+)$/);
    if (m) {
      const num = m[1] ?? '';
      const base = m[2] ?? code;
      const name = GRAMMAR_TIPS[base] ?? base;
      return `${num}-place ${name.toLowerCase()}`;
    }
    return GRAMMAR_TIPS[code] ?? code;
  }
  const TAG_TIPS: Record<string, string> = {
    B: 'B (beu) — object: patient, part, property',
    C: 'C (cau) — quantity: amount, value',
    D: 'D (dio) — direction: recipient, destination',
    F: 'F (foa) — whole: set, collectivity',
    G: 'G (goa) — greater in a comparison',
    J: 'J (jui) — lesser in a comparison',
    K: 'K (kao) — actor: agent, doer',
    N: 'N (neu) — condition: field, circumstance',
    P: 'P (pou) — product: output, purpose',
    S: 'S (sau) — source: origin, reason, cause',
    V: 'V (veu) — event: means, route, effect',
  };
  function tagsTip(tagStr: string): string {
    return (
      tagStr
        // square brackets aren't allowed inside tag strings
        .replace(/\[|\]/g, '')
        .split('')
        .filter((c) => /[A-Z]/.test(c))
        .map((c) => TAG_TIPS[c] ?? c)
        .join('\n')
    );
  }
  const USAGE_TIPS: Record<string, string> = {
    le: 'le — specific instance: "the X I have in mind"',
    lo: 'lo — mass term: X-stuff in general',
    la: 'la — used as a name',
    lio: 'lio — used as a number',
    nu: 'nu — swap 1st & 2nd argument (conversion)',
    fu: 'fu — move 3rd argument to front',
    ju: 'ju — move 4th argument to front',
    po: 'po — event abstraction: "the event of …"',
    pu: 'pu — property abstraction: "the property of being …"',
    zo: 'zo — quantity abstraction: "the amount of …"',
    lopo: 'lopo — mass term of event',
    lepo: 'lepo — specific event',
    lopu: 'lopu — mass term of property',
    lepu: 'lepu — specific property',
  };
  function usageTip(u: string | null | undefined): string | null {
    if (!u) return null;
    const key = u
      .replace(/[\s—%-].*$/, '')
      .trim()
      .toLowerCase();
    return USAGE_TIPS[key] ?? null;
  }

  import { renderBody } from '../text';
  import type { WordDetail, Definition } from '../../types';

  const { word, loading = false }: { word: WordDetail; loading?: boolean } = $props();

  let editingDef = $state<number | null>(null);
  let newDef = $state(false);
  let defForm = $state({ grammar: '', usage: '', body: '', tags: '' });
  let confirmDel = $state(false);
  let usedInOpen = $state(true);
  let activeTooltip = $state<{ content: string; x: number; y: number } | null>(null);

  function handleTooltipClick(e: MouseEvent | KeyboardEvent, content: string) {
    if (app.currentPlatform === 'android' && app.prefs.showTooltips) {
      e.preventDefault();
      const target = e.target as HTMLElement;
      const rect = target.getBoundingClientRect();
      activeTooltip = {
        content,
        x: rect.left + rect.width / 2,
        y: rect.top - 10,
      };
    }
  }

  function closeTooltip() {
    activeTooltip = null;
  }

  function startEditDef(d: Definition) {
    editingDef = d.id;
    newDef = false;
    defForm = { grammar: d.grammar || '', usage: d.usage || '', body: d.body, tags: d.tags || '' };
  }
  function startNewDef() {
    newDef = true;
    editingDef = null;
    defForm = { grammar: '', usage: '', body: '', tags: '' };
  }
  function cancelDef() {
    editingDef = null;
    newDef = false;
  }
  async function submitDef() {
    if (!defForm.body.trim()) return;
    await saveDef(editingDef, word.id, {
      grammar: defForm.grammar || null,
      usage: defForm.usage || null,
      body: defForm.body,
      tags: defForm.tags || null,
    });
    editingDef = null;
    newDef = false;
  }

  function handleXref(e: Event) {
    const t = e.target as HTMLElement;
    if (t.classList.contains('xref')) {
      const found = app.words.find((w) => w.name === t.dataset.word);
      if (found) selectWord(found.id);
    }
  }
  function clickAffix(a: string) {
    // Try to find exact match first (like xref)
    const found = app.words.find((w) => w.name === a);
    if (found) {
      selectWord(found.id);
    } else {
      // Fallback to search if no exact match
      app.tab = 'words';
      app.searchQ = a;
      applyFilter();
      if (app.filteredWords.length > 0 && app.filteredWords[0]) {
        selectWord(app.filteredWords[0].id);
      }
    }
  }

  // Build visible meta chips once per word change.
  const visibleMeta = $derived(
    [
      { key: 'type', label: 'Type', val: word.type_name },
      { key: 'source', label: 'Src', val: word.source },
      { key: 'year', label: 'Year', val: word.year },
      { key: 'rank', label: 'Rank', val: word.rank },
      { key: 'match', label: 'Match', val: word.match_ },
      { key: 'event', label: 'From', val: word.event_start_name },
      { key: 'until', label: 'Until', val: word.event_end_name },
    ].filter((f) => app.prefs.visibleMeta.includes(f.key) && f.val)
  );
</script>

<article class="wd" class:wd-loading={loading}>
  <!-- ── HEAD ────────────────────────────────────────────────────── -->
  <div class="wd-head">
    <div class="wd-name">{word.name}</div>
    {#if word.affixes.length || word.spellings.length}
      <div class="wd-badges">
        {#each word.affixes as a}
          <button class="ui-word af-chip" onclick={() => clickAffix(a)}>{a}</button>
        {/each}
        {#each word.spellings as s}
          <span class="badge bd-spell">{s}</span>
        {/each}
      </div>
    {/if}
    <div
      class="wd-acts"
      style:visibility={app.readonly ? 'hidden' : 'visible'}
      aria-hidden={app.readonly}
    >
      <button
        class="btn btn-ic btn-au"
        title="Edit word"
        onclick={() => {
          app.panel = 'word-form';
          app.editing = true;
        }}><Icon name="edit" size={app.currentPlatform === 'android' ? 18 : 16} /></button
      >
      <button class="btn btn-ic btn-r" title="Delete word" onclick={() => (confirmDel = true)}
        ><Icon name="delete" size={app.currentPlatform === 'android' ? 18 : 16} /></button
      >
    </div>
  </div>

  <!-- ── META CHIPS ─────────────────────────────────────────────── -->
  {#if visibleMeta.length > 0}
    <div class="meta-row">
      {#each visibleMeta as f}
        <div class="meta-chip">
          <span class="mc-lbl">{f.label}</span>
          <span class="mc-val">{f.val}</span>
        </div>
      {/each}
    </div>
  {/if}

  <!-- ── ORIGINS ────────────────────────────────────────────────── -->
  {#if word.origin || word.origin_x}
    <div class="origin-box">
      <div class="sec-title">Origins</div>
      <div class="origin-text">{word.origin || ''}{word.origin_x ? ` = ${word.origin_x}` : ''}</div>
    </div>
  {/if}

  <!-- ── DEFINITIONS ───────────────────────────────────────────── -->
  <!-- sec-row: symmetric padding so + Add button has equal space top/bottom from divider line -->
  <div class="sec-row">
    <span class="sec-title">Definitions</span>
    <button
      class="btn btn-g btn-sm"
      style:visibility={app.readonly ? 'hidden' : 'visible'}
      aria-hidden={app.readonly}
      onclick={startNewDef}>+ Add</button
    >
  </div>

  <ol class="def-list">
    {#each word.definitions as d}
      {#if editingDef === d.id}
        <li class="def-item def-form">{@render defEditor()}</li>
      {:else}
        <li class="def-item">
          <div class="def-head">
            {#if d.usage}
              {@const tip = app.prefs.showTooltips ? usageTip(d.usage) : null}
              <span
                class="def-usage"
                class:has-tip={!!tip}
                title={tip ?? undefined}
                role="button"
                tabindex="0"
                onclick={(e) => handleTooltipClick(e, tip || '')}
                onkeydown={(e) => e.key === 'Enter' && handleTooltipClick(e, tip || '')}
              >
                {d.usage.replace('%', word.name)}
              </span>
            {/if}
            {#if d.grammar}
              {@const tip = app.prefs.showTooltips ? grammarTip(d.grammar) : null}
              <span
                class="def-grammar"
                class:has-tip={!!tip}
                title={tip ?? undefined}
                role="button"
                tabindex="0"
                onclick={(e) => handleTooltipClick(e, tip || '')}
                onkeydown={(e) => e.key === 'Enter' && handleTooltipClick(e, tip || '')}
              >
                ({d.grammar})
              </span>
            {/if}
            {#if d.tags}
              {@const tip = app.prefs.showTooltips ? tagsTip(d.tags) : null}
              <span
                class="def-tags"
                class:has-tip={!!tip}
                title={tip ?? undefined}
                role="button"
                tabindex="0"
                onclick={(e) => handleTooltipClick(e, tip || '')}
                onkeydown={(e) => e.key === 'Enter' && handleTooltipClick(e, tip || '')}
              >
                [{d.tags}]
              </span>
            {/if}
            <!-- btn-ic: fixed 26×26px square regardless of glyph width -->
            <div
              class="def-acts"
              style:visibility={app.readonly ? 'hidden' : 'visible'}
              aria-hidden={app.readonly}
            >
              <button class="btn btn-ic btn-ghost" onclick={() => startEditDef(d)}
                ><Icon name="edit" size={app.currentPlatform === 'android' ? 18 : 16} /></button
              >
              <button class="btn btn-ic btn-ghost btn-r" onclick={() => deleteDef(d.id, word.id)}
                ><Icon name="delete" size={app.currentPlatform === 'android' ? 18 : 16} /></button
              >
            </div>
          </div>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="def-body" onclick={handleXref} onkeydown={handleXref}>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html renderBody(d.body)}
          </div>
        </li>
      {/if}
    {/each}
    {#if newDef}<li class="def-item def-form">{@render defEditor()}</li>{/if}
  </ol>

  <!-- ── NOTES ──────────────────────────────────────────────────── -->
  {#if word.notes}
    <div class="sec-row"><span class="sec-title">Notes</span></div>
    <div class="notes-text">{word.notes}</div>
  {/if}

  <!-- ── USED IN ────────────────────────────────────────────────── -->
  {#if word.used_in.length > 0}
    <div class="sec-row no-rule">
      <button class="sec-toggle" onclick={() => (usedInOpen = !usedInOpen)}>
        <span class="sec-title">Used In</span>
        <span class="sec-cnt">{word.used_in.length}</span>
        <span class="sec-arrow" class:open={usedInOpen}>›</span>
      </button>
    </div>
    {#if usedInOpen}
      <div class="used-in">
        {#each word.used_in as w}
          <button
            class="ui-word"
            onclick={() => {
              const f = app.words.find((x) => x.name === w);
              if (f) selectWord(f.id);
            }}
          >
            {w}
          </button>
        {/each}
      </div>
    {/if}
  {/if}

  <!-- Tooltip для Android -->
  {#if activeTooltip && app.currentPlatform === 'android'}
    <div
      class="tooltip-popup"
      role="dialog"
      aria-label="Tooltip"
      style="position: fixed; left: {activeTooltip.x}px; top: {activeTooltip.y}px; transform: translateX(-50%);"
      onclick={closeTooltip}
      onkeydown={(e) => e.key === 'Escape' && closeTooltip()}
      tabindex="-1"
    >
      <div class="tooltip-content">
        {#each activeTooltip.content.split('\n') as line}
          {line}<br />
        {/each}
      </div>
    </div>
  {/if}
</article>

{#if confirmDel}
  <div
    class="modal-overlay"
    onclick={() => (confirmDel = false)}
    onkeydown={(e) => e.key === 'Escape' && (confirmDel = false)}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="modal" role="presentation" onclick={(e) => e.stopPropagation()}>
      <div class="modal-title">Delete "{word.name}"?</div>
      <p class="modal-body">This will also delete all definitions. Cannot be undone.</p>
      <div class="modal-actions">
        <button class="btn" onclick={() => (confirmDel = false)}>Cancel</button>
        <button
          class="btn btn-r"
          onclick={() => {
            confirmDel = false;
            deleteWord(word.id);
          }}>Delete</button
        >
      </div>
    </div>
  </div>
{/if}

{#snippet defEditor()}
  <div class="def-editor">
    <div class="form-row">
      <div class="fg">
        <label for="def-grammar">Grammar</label>
        <input id="def-grammar" class="fi" bind:value={defForm.grammar} placeholder="2a, n, av…" />
      </div>
      <div class="fg">
        <label for="def-usage">Usage</label>
        <input id="def-usage" class="fi" bind:value={defForm.usage} placeholder="% or pu %" />
      </div>
      <div class="fg">
        <label for="def-tags">Tags</label>
        <input id="def-tags" class="fi" bind:value={defForm.tags} placeholder="G-J" />
      </div>
    </div>
    <div class="fg" style="margin-bottom:.5rem">
      <label for="def-body">Body</label>
      <textarea id="def-body" class="fta" bind:value={defForm.body} rows="3"></textarea>
    </div>
    <div style="display:flex;gap:.35rem">
      <button class="btn btn-g btn-sm" onclick={submitDef}>Save</button>
      <button class="btn btn-sm" onclick={cancelDef}>Cancel</button>
    </div>
  </div>
{/snippet}

<style>
  /* ── Word header ── */
  .wd {
    padding: 0.75rem 0 0;
    transition: opacity 130ms;
  }
  .wd-loading {
    opacity: 0.4;
    pointer-events: none;
  }
  .wd-head {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 0.75rem;
  }
  .wd-name {
    font-size: var(--fs-xl);
    font-weight: 700;
    color: var(--text);
    font-style: italic;
    line-height: 1.2;
  }
  .wd-badges {
    display: flex;
    gap: 0.28rem;
    flex-wrap: wrap;
    align-items: center;
    flex: 1;
    padding-top: 0.2rem; /* optical alignment with name baseline */
  }
  .wd-acts {
    display: flex;
    gap: 0.22rem;
    margin-left: auto;
    flex-shrink: 0;
    padding-top: 0.15rem;
  }

  /* ── Meta chips ── */
  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.28rem;
    margin-bottom: 0.75rem;
  }
  .meta-chip {
    display: inline-flex;
    flex-direction: column;
    gap: 0.08rem;
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.25rem 0.52rem;
    min-width: 0;
  }
  /* mc-lbl and mc-val are global classes from App.svelte */
  .meta-chip :global(.mc-val) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  /* ── Origins ── */
  .origin-box {
    background: var(--surf2);
    border: 1px solid var(--border);
    border-left: 2px solid var(--border3);
    border-radius: var(--r-md);
    padding: 0.5rem 0.85rem;
    margin-bottom: 0.75rem;
  }
  .origin-box .sec-title {
    margin-bottom: 0.2rem;
  }
  .origin-text {
    font-size: var(--fs-sm);
    color: var(--text2);
    font-style: italic;
    line-height: 1.5;
  }

  /* ── Definitions ── */
  /* sec-row is global: symmetric padding keeps button away from border line */
  .def-list {
    list-style: decimal;
    padding-left: 1.4rem;
    margin-top: 0.38rem;
  }
  .def-item {
    padding: 0.32rem 0 0.32rem 0.2rem;
    border-bottom: 1px solid var(--border);
  }
  .def-item:last-child {
    border-bottom: none;
  }

  .def-head {
    display: flex;
    align-items: center;
    gap: 0.28rem;
    flex-wrap: wrap;
    margin-bottom: 0.12rem;
    min-height: 26px; /* match btn-ic height */
  }
  .def-usage {
    font-size: var(--fs-sm);
    color: var(--gold);
    font-weight: 600;
  }
  .def-grammar {
    font-size: var(--fs-sm);
    color: var(--teal);
  }
  .def-tags {
    font-size: var(--fs-xs);
    color: var(--text3);
  }
  .has-tip {
    cursor: help;
  }

  /* ── Def action buttons ── */
  /* margin-left:auto pushes to right; gap:.1rem between edit+delete */
  .def-acts {
    margin-left: auto;
    display: flex;
    gap: 0.1rem;
    opacity: 0;
    transition: opacity 140ms;
    flex-shrink: 0;
  }
  .def-item:hover .def-acts {
    opacity: 1;
  }
  @media (max-width: 640px) {
    .def-acts {
      opacity: 1;
    }
  }

  .def-body {
    font-size: var(--fs-base);
    color: var(--text);
    line-height: 1.65;
  }
  .def-form {
    list-style: none;
    margin-left: -1.4rem;
    padding: 0.4rem 0.2rem;
  }
  .def-editor {
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    padding: 0.65rem 0.75rem;
  }

  /* ── Notes / Used-in ── */
  .notes-text {
    font-size: var(--fs-base);
    color: var(--text2);
    padding: 0.4rem 0.52rem;
    background: var(--surf2);
    border-radius: var(--r-md);
    margin-top: 0.35rem;
    line-height: 1.65;
  }
  .used-in {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.35rem;
  }
  .ui-word {
    padding: 3px 8px;
    border-radius: var(--r-sm);
    font-size: var(--fs-xs);
    font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
    cursor: pointer;
    background: var(--surf2);
    border: 1px solid var(--border);
    color: var(--text2);
    transition: all 140ms;
    -webkit-tap-highlight-color: transparent;
  }
  .ui-word:hover {
    color: var(--gold);
    border-color: var(--gold-d);
    background: var(--gold-g);
  }
  .af-chip {
    background: var(--green-g);
    border-color: var(--green-d);
    color: var(--green);
  }
  .af-chip:hover {
    background: var(--green-g);
    border-color: var(--green);
    color: var(--green);
    filter: brightness(1.15);
  }

  /* ── Used In collapsible header ── */
  /* sec-row is global; toggle button fills it */
  .sec-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    font: inherit;
    -webkit-tap-highlight-color: transparent;
  }
  .sec-toggle:hover .sec-title {
    opacity: 0.75;
  }
  /* extend the divider line after the arrow */
  .sec-toggle::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .sec-cnt {
    font-size: var(--fs-label);
    font-weight: 700;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    background: var(--surf2);
    color: var(--text2);
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
  .sec-arrow {
    font-size: 0.8rem;
    color: var(--text3);
    flex-shrink: 0;
    transform: rotate(90deg);
    transition: transform 180ms ease;
    display: inline-block;
    line-height: 1;
  }
  .sec-arrow.open {
    transform: rotate(-90deg);
  }
  /* row whose sec-toggle already draws the rule via ::after */
  .sec-row.no-rule {
    border-bottom: none;
  }

  /* Tooltip для Android */
  .tooltip-popup {
    background: var(--surf);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    box-shadow: 0 4px 12px var(--shd);
    padding: 0.5rem;
    max-width: 200px;
    z-index: 1000;
    font-size: 0.7rem;
    line-height: 1.4;
    color: var(--text);
  }

  .tooltip-content {
    white-space: pre-wrap;
  }
</style>
