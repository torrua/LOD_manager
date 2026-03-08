<script lang="ts">
  import { save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { open as openFilePicker } from '@tauri-apps/plugin-dialog';
  import { open as openDbDialog } from '@tauri-apps/plugin-dialog';
  import {
    app,
    openDb,
    createDb,
    closeDb,
    importFiles,
    exportHtmlToFile,
    toast,
    loadDbStats,
    setPref,
    toggleMetaField,
    rebuildFts,
    loadWords,
  } from '../store.svelte';
  import type { ImportResult } from '../../types';

  // ── Import ────────────────────────────────────────────────────────────────
  let impPaths = $state<string[]>([]);
  let impResult = $state<ImportResult | null>(null);
  let impRunning = $state(false);

  async function pickImport() {
    const sel = await openFilePicker({
      multiple: true,
      title: 'Select LOD text files',
      filters: [{ name: 'Text', extensions: ['txt'] }],
    });
    if (sel) {
      impPaths = Array.isArray(sel) ? sel : [sel];
      impResult = null;
    }
  }
  function rmImp(p: string) {
    impPaths = impPaths.filter((x) => x !== p);
  }
  function bn(p: string) {
    return p.split(/[/\\]/).pop() || p;
  }
  async function runImport() {
    if (!impPaths.length || impRunning) return;
    impRunning = true;
    impResult = null;
    try {
      impResult = (await importFiles(impPaths)) as ImportResult;
      toast(`Import: ${impResult.words} words`, impResult.errors ? 'err' : 'ok');
    } catch (e) {
      toast(String(e), 'err');
    } finally {
      impRunning = false;
    }
  }

  // ── Export — issue #4 ─────────────────────────────────────────────────────
  // Direction: 'le' = Loglan→English (standard), 'el' = English→Loglan (reversed)
  // For now both use same HTML structure; direction saved for future use
  let expEvent = $state('');
  let expDir = $state<'le' | 'el'>('le');
  let expRunning = $state(false);

  function buildExportName(): string {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    const date = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`;
    const time = `${pad(now.getHours())}${pad(now.getMinutes())}`;
    const dir = expDir === 'el' ? 'E_to_L' : 'L_to_E';
    const ev = expEvent ? `_${expEvent.replace(/\s+/g, '_')}` : '';
    return `LOD_${dir}${ev}_${date}_${time}.html`;
  }

  async function runExport() {
    if (expRunning || !app.dbOpen) return;
    const dest = await saveDialog({
      title: 'Save HTML Dictionary',
      defaultPath: buildExportName(),
      filters: [{ name: 'HTML', extensions: ['html'] }],
    });
    if (!dest) return;
    expRunning = true;
    try {
      await exportHtmlToFile(dest, expEvent || null);
      toast(`Exported → ${bn(dest)}`, 'ok');
      app.toolsOpen = false;
    } catch (e) {
      toast(String(e), 'err');
    } finally {
      expRunning = false;
    }
  }

  // ── Database ──────────────────────────────────────────────────────────────
  async function handleSwitch() {
    const p = await openDbDialog({
      title: 'Open LOD Database',
      filters: [{ name: 'SQLite', extensions: ['db', 'sqlite', 'sqlite3'] }],
    });
    if (p) await openDb(p as string).catch((e) => toast(String(e), 'err'));
  }
  async function handleNew() {
    const p = await saveDialog({
      title: 'Create New LOD Database',
      defaultPath: 'loglan.db',
      filters: [{ name: 'SQLite', extensions: ['db'] }],
    });
    if (p) await createDb(p).catch((e) => toast(String(e), 'err'));
  }

  // lazy-load stats when Database tab opens
  $effect(() => {
    if (app.toolsOpen && app.toolsTab === 'database' && app.dbOpen && !app.dbStats)
      loadDbStats().catch(() => {});
  });

  // ── Settings meta fields ──────────────────────────────────────────────────
  const META_OPTS = [
    { key: 'type', label: 'Type' },
    { key: 'source', label: 'Source' },
    { key: 'year', label: 'Year' },
    { key: 'rank', label: 'Rank' },
    { key: 'match', label: 'Match %' },
    { key: 'event', label: 'From (event)' },
    { key: 'until', label: 'Until (event)' },
  ];
</script>

<button class="td-backdrop" onclick={() => (app.toolsOpen = false)} aria-label="Close tools"
></button>

<div class="td">
  <div class="td-hdr">
    <span class="td-title">Tools</span>
    <button class="btn btn-icon btn-ghost" onclick={() => (app.toolsOpen = false)}>✕</button>
  </div>

  <nav class="td-tabs">
    {#each ['database', 'import', 'export', 'settings'] as const as t}
      <button class="td-tab" class:on={app.toolsTab === t} onclick={() => (app.toolsTab = t)}>
        {t.charAt(0).toUpperCase() + t.slice(1)}
      </button>
    {/each}
  </nav>

  <div class="td-body">
    <!-- ── DATABASE ── -->
    {#if app.toolsTab === 'database'}
      {#if app.dbOpen}
        <div class="db-stat-grid">
          <span class="dsl">File</span>
          <span class="dsv" title={app.dbPath}>{app.dbPath.split(/[/\\]/).pop()}</span>
          {#if app.dbStats}
            <span class="dsl">Words</span>
            <span class="dsv">{app.dbStats.word_count.toLocaleString()}</span>
            <span class="dsl">Definitions</span><span class="dsv"
              >{app.dbStats.definition_count.toLocaleString()}</span
            >
            <span class="dsl">Events</span> <span class="dsv">{app.dbStats.event_count}</span>
            <span class="dsl">Types</span> <span class="dsv">{app.dbStats.type_count}</span>
            <span class="dsl">Authors</span> <span class="dsv">{app.dbStats.author_count}</span>
            <span class="dsl">Affixes</span> <span class="dsv">{app.dbStats.affix_count}</span>
            <span class="dsl">Spellings</span> <span class="dsv">{app.dbStats.spelling_count}</span>
            {#if app.dbStats.settings.length > 0}
              <span class="dsl">Settings</span><span class="dsv"
                >{app.dbStats.settings.length} keys</span
              >
              {#each app.dbStats.settings.slice(0, 8) as s}
                <span class="dsl dsl-sub">{s.key}</span>
                <span class="dsv dsv-sub">{s.value}</span>
              {/each}
            {/if}
          {:else}
            <span class="dsl">Words</span><span class="dsv">{app.wordCount.toLocaleString()}</span>
          {/if}
        </div>
        <!-- issue #1: no ellipsis on buttons -->
        <div class="td-acts">
          <button class="btn btn-au btn-sm" onclick={handleSwitch}>⎘ Switch DB</button>
          <button class="btn btn-g btn-sm" onclick={handleNew}>＋ New DB</button>
          <button class="btn btn-r btn-sm" onclick={closeDb}>✕ Close</button>
        </div>
      {:else}
        <p class="td-hint">No database open.</p>
        <div class="td-acts">
          <button class="btn btn-au btn-lg" onclick={handleSwitch}>Open DB</button>
          <button class="btn btn-g btn-lg" onclick={handleNew}>New DB</button>
        </div>
      {/if}

      <!-- ── IMPORT ── -->
    {:else if app.toolsTab === 'import'}
      <p class="td-hint">
        Select LOD text files: <code>Word.txt</code>, <code>WordDef.txt</code>,
        <code>WordSpell.txt</code>, <code>LexEvent.txt</code>, <code>type.txt</code>,
        <code>author.txt</code>, <code>settings.txt</code>.
      </p>
      <div class="file-zone" class:has={impPaths.length > 0}>
        {#if impPaths.length === 0}
          <button class="btn btn-au btn-sm" onclick={pickImport}>Browse files…</button>
        {:else}
          {#each impPaths as p}
            <div class="file-row">
              <span class="f-name">{bn(p)}</span>
              <button class="btn btn-icon btn-sm btn-ghost btn-r" onclick={() => rmImp(p)}>×</button
              >
            </div>
          {/each}
          <button class="btn btn-sm" style="margin-top:.3rem" onclick={pickImport}>Add more…</button
          >
        {/if}
      </div>
      {#if impPaths.length > 0}
        <button
          class="btn btn-g btn-sm"
          style="margin-top:.5rem"
          onclick={runImport}
          disabled={impRunning || !app.dbOpen}
        >
          {impRunning
            ? 'Importing…'
            : `Import ${impPaths.length} file${impPaths.length > 1 ? 's' : ''}`}
        </button>
      {/if}
      {#if !app.dbOpen}
        <p class="td-hint td-warn">Open a database first.</p>
      {/if}
      {#if impResult}
        <div class="imp-res" class:err={impResult.errors > 0}>
          <b>{impResult.errors ? '⚠' : '✓'}</b>
          Words: {impResult.words} · Defs: {impResult.definitions} · Events: {impResult.events} · Settings:
          {impResult.settings} · Errors: {impResult.errors}
        </div>
      {/if}

      <!-- ── EXPORT ── -->
    {:else if app.toolsTab === 'export'}
      <p class="td-hint">Generate a standalone HTML dictionary file.</p>
      <!-- issue #4: direction selector -->
      <div class="fg" style="margin-bottom:.55rem">
        <p class="td-field-label" id="dir-label">Direction</p>
        <div class="dir-row" role="group" aria-labelledby="dir-label">
          <button class="dir-btn" class:on={expDir === 'le'} onclick={() => (expDir = 'le')}>
            Loglan → English
          </button>
          <button class="dir-btn" class:on={expDir === 'el'} onclick={() => (expDir = 'el')}>
            English → Loglan
          </button>
        </div>
      </div>
      <div class="fg" style="margin-bottom:.65rem">
        <label for="td-exp-event">Filter by event <span style="opacity:.5">(optional)</span></label>
        <select id="td-exp-event" class="fsel" bind:value={expEvent}>
          <option value="">All words</option>
          {#each app.events as ev}
            <option value={ev.name}>{ev.name}</option>
          {/each}
        </select>
      </div>
      <button class="btn btn-au" onclick={runExport} disabled={expRunning || !app.dbOpen}>
        {expRunning ? 'Generating…' : '⬇ Export HTML…'}
      </button>
      {#if !app.dbOpen}
        <p class="td-hint td-warn" style="margin-top:.4rem">Open a database first.</p>
      {/if}

      <!-- ── SETTINGS ── -->
    {:else if app.toolsTab === 'settings'}
      <!-- Word list event filter -->
      <div class="settings-group">
        <div class="sg-title">Word List Filter</div>
        <label class="sg-label" for="td-ev-filter">Show words active at event</label>
        <select
          id="td-ev-filter"
          class="td-select"
          value={app.prefs.eventFilter ?? ''}
          onchange={async (e) => {
            const v = (e.target as HTMLSelectElement).value;
            setPref('eventFilter', v ? parseInt(v) : null);
            await loadWords();
          }}
        >
          <option value="">— All words —</option>
          {#each app.events as ev}
            <option value={ev.id}>{ev.name}{ev.annotation ? ' · ' + ev.annotation : ''}</option>
          {/each}
        </select>
        <p class="td-hint" style="margin:.25rem 0 0">
          Hides words added after the selected event, and words that were removed by it.
        </p>
      </div>

      <!-- Tooltips -->
      <div class="settings-group">
        <div class="sg-title">Definitions Display</div>
        <label class="ck-row">
          <input
            type="checkbox"
            class="ck"
            checked={app.prefs.showTooltips}
            onchange={(e) => setPref('showTooltips', (e.target as HTMLInputElement).checked)}
          />
          Show tooltips on grammar codes &amp; usages
        </label>
        <p class="td-hint" style="margin:.2rem 0">
          Hover over <code>(2v)</code>, <code>[G-J]</code>, <code>lo —</code> to see explanations.
        </p>
      </div>

      <div class="settings-group">
        <div class="sg-title">Sidebar List</div>
        <label class="ck-row">
          <input
            type="checkbox"
            class="ck"
            checked={app.prefs.showTypeTag}
            onchange={(e) => setPref('showTypeTag', (e.target as HTMLInputElement).checked)}
          />
          Show word type tag
        </label>
        <label class="ck-row">
          <input
            type="checkbox"
            class="ck"
            checked={app.prefs.showDefCount}
            onchange={(e) => setPref('showDefCount', (e.target as HTMLInputElement).checked)}
          />
          Show definition count
        </label>
      </div>

      <!-- Meta field visibility -->
      <div class="settings-group">
        <div class="sg-title">Visible Metadata Fields</div>
        {#each META_OPTS as opt}
          <label class="ck-row">
            <input
              type="checkbox"
              class="ck"
              checked={app.prefs.visibleMeta.includes(opt.key)}
              onchange={() => toggleMetaField(opt.key)}
            />
            {opt.label}
          </label>
        {/each}
        {#if app.prefs.visibleMeta.length === 0}
          <p class="td-hint td-warn" style="margin-top:.3rem">
            No fields selected — metadata hidden.
          </p>
        {/if}
      </div>

      <!-- E→L search results display -->
      <div class="settings-group">
        <div class="sg-title">E→L Search Results</div>
        <label class="ck-row">
          <input
            type="checkbox"
            class="ck"
            checked={app.prefs.elShowDetails}
            onchange={(e) => setPref('elShowDetails', (e.target as HTMLInputElement).checked)}
          />
          Show result details
        </label>
        <div class="ck-indent" class:ck-disabled={!app.prefs.elShowDetails}>
          <label class="ck-row">
            <input
              type="checkbox"
              class="ck"
              disabled={!app.prefs.elShowDetails}
              checked={app.prefs.elShowSnippet}
              onchange={(e) => setPref('elShowSnippet', (e.target as HTMLInputElement).checked)}
            />
            Definition snippet
          </label>
          <label class="ck-row">
            <input
              type="checkbox"
              class="ck"
              disabled={!app.prefs.elShowDetails}
              checked={app.prefs.elShowGrammar}
              onchange={(e) => setPref('elShowGrammar', (e.target as HTMLInputElement).checked)}
            />
            Grammar code (n, 2v…)
          </label>
          <label class="ck-row">
            <input
              type="checkbox"
              class="ck"
              disabled={!app.prefs.elShowDetails}
              checked={app.prefs.elShowType}
              onchange={(e) => setPref('elShowType', (e.target as HTMLInputElement).checked)}
            />
            Word type tag
          </label>
          <label class="ck-row">
            <input
              type="checkbox"
              class="ck"
              disabled={!app.prefs.elShowDetails}
              checked={app.prefs.elShowCount}
              onchange={(e) => setPref('elShowCount', (e.target as HTMLInputElement).checked)}
            />
            Match count badge
          </label>
        </div>
      </div>

      <!-- E→L search engine -->
      <div class="settings-group">
        <div class="sg-title">E→L Search Engine</div>
        <label class="ck-row">
          <input
            type="checkbox"
            class="ck"
            checked={app.prefs.elUseLike}
            onchange={(e) => setPref('elUseLike', (e.target as HTMLInputElement).checked)}
          />
          Use LIKE instead of FTS5
        </label>
        <p class="td-hint" style="margin:.2rem 0">
          FTS5 is faster and supports prefix / phrase matching.
        </p>
        <div style="display:flex;align-items:center;gap:.5rem;margin-top:.4rem">
          <button class="btn btn-sm" onclick={() => rebuildFts()}>Rebuild FTS Index</button>
          {#if app.elFtsReady}
            <span style="font-size:.6rem;color:var(--green,#5a8)">✓ ready</span>
          {:else}
            <span style="font-size:.6rem;color:var(--red,#c44)">⚠ not built</span>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .td-select {
    width: 100%;
    padding: 0.3rem 0.5rem;
    font-size: var(--fs-sm);
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    color: var(--text);
    cursor: pointer;
    margin-top: 0.25rem;
    font-family: inherit;
  }
  .sg-label {
    font-size: var(--fs-xs);
    color: var(--text2);
    display: block;
  }
  .td-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 200;
  }
  .td {
    position: fixed;
    right: 0;
    top: 0;
    bottom: 0;
    width: 310px;
    max-width: 94vw;
    background: var(--surf);
    border-left: 1px solid var(--border);
    z-index: 201;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 24px var(--shd-lg);
    animation: td-in 160ms ease;
  }
  @keyframes td-in {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }
  @media (max-width: 640px) {
    .td {
      right: 0;
      left: 0;
      top: auto;
      bottom: 0;
      width: 100%;
      max-width: 100%;
      border-left: none;
      border-top: 1px solid var(--border);
      border-radius: 14px 14px 0 0;
      max-height: 90vh;
      min-height: 50vh;
      animation: td-up 200ms ease;
    }
    @keyframes td-up {
      from {
        transform: translateY(100%);
      }
      to {
        transform: translateY(0);
      }
    }
  }
  .td-hdr {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.55rem 0.85rem 0.4rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .td-title {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text);
    letter-spacing: 0.04em;
  }
  .td-tabs {
    display: flex;
    gap: 2px;
    padding: 0.24rem 0.36rem 0.16rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .td-tab {
    flex: 1;
    height: 26px;
    font-size: 0.58rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text2);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    cursor: pointer;
    font-family: inherit;
    transition: all 140ms;
    padding: 0;
  }
  .td-tab:hover:not(.on) {
    background: var(--surf2);
    border-color: var(--border);
  }
  .td-tab.on {
    color: var(--gold);
    border-color: var(--gold-d);
    background: var(--gold-g);
  }
  .td-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .td-field-label {
    font-size: var(--fs-label);
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    margin-bottom: 0.22rem;
    display: block;
  }
  .td-hint {
    font-size: 0.7rem;
    color: var(--text2);
    line-height: 1.6;
  }
  .td-hint code {
    color: var(--gold);
    background: var(--gold-g);
    padding: 0 3px;
    border-radius: 2px;
  }
  .td-warn {
    color: var(--red);
    margin-top: 0.3rem;
  }
  .td-acts {
    display: flex;
    flex-wrap: wrap;
    gap: 0.32rem;
  }

  /* issue #2: stats grid */
  .db-stat-grid {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.12rem 0.6rem;
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    padding: 0.6rem 0.75rem;
    margin-bottom: 0.45rem;
  }
  .dsl {
    font-size: 0.52rem;
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    display: flex;
    align-items: center;
  }
  .dsv {
    font-size: 0.73rem;
    color: var(--text);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
  }
  .dsl-sub {
    font-size: 0.46rem;
    padding-left: 0.5rem;
    color: var(--text3);
  }
  .dsv-sub {
    font-size: 0.63rem;
    color: var(--text2);
  }

  /* export direction */
  .dir-row {
    display: flex;
    gap: 0.28rem;
    margin-top: 0.2rem;
  }
  .dir-btn {
    flex: 1;
    height: 28px;
    font-size: 0.65rem;
    font-family: inherit;
    border-radius: var(--r-md);
    background: var(--surf2);
    border: 1px solid var(--border);
    color: var(--text2);
    cursor: pointer;
    transition: all 140ms;
  }
  .dir-btn.on {
    background: var(--gold-g);
    border-color: var(--gold-d);
    color: var(--gold);
  }
  .dir-btn:hover:not(.on) {
    background: var(--surf3);
    border-color: var(--border3);
  }

  /* import */
  .file-zone {
    border: 1px dashed var(--border2);
    border-radius: var(--r-lg);
    padding: 0.5rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-height: 48px;
    justify-content: center;
    align-items: flex-start;
  }
  .file-zone.has {
    align-items: stretch;
  }
  .file-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.16rem 0.38rem;
  }
  .f-name {
    font-size: 0.68rem;
    color: var(--gold);
    font-weight: 600;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .imp-res {
    font-size: 0.7rem;
    padding: 0.42rem 0.55rem;
    border-radius: var(--r-md);
    background: var(--green-g);
    border: 1px solid var(--green-d);
    color: var(--green);
  }
  .imp-res.err {
    background: var(--red-g);
    border-color: var(--red-d);
    color: var(--red);
  }

  /* settings */
  .ck-indent {
    margin-left: 1.3rem;
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
  }
  .ck-disabled {
    opacity: 0.4;
    pointer-events: none;
  }
  .settings-group {
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    padding: 0.6rem 0.75rem;
  }
  .sg-title {
    font-size: 0.58rem;
    font-weight: 700;
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.42rem;
  }
  .ck-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.73rem;
    color: var(--text2);
    padding: 0.22rem 0;
    cursor: pointer;
    user-select: none;
  }
  .ck-row:hover {
    color: var(--text);
  }
  .ck {
    accent-color: var(--gold);
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    cursor: pointer;
  }
</style>
