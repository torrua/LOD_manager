<script lang="ts">
  import {
    app,
    applyFilter,
    selectWord,
    selectEvent,
    setSearchMode,
    searchEnglishDebounced,
    searchEnglishNow,
    setPref,
  } from '../store.svelte';
  import ELResults from './ELResults.svelte';
  import Icon from './Icon.svelte';

  // ── Resize ────────────────────────────────────────────────────────────────
  let sbWidth = $state(parseInt(localStorage.getItem('sb-w') || '252'));
  let dragging = $state(false);
  function dragStart(e: MouseEvent) {
    dragging = true;
    const sx = e.clientX,
      sw = sbWidth;
    const mm = (e2: MouseEvent) => {
      sbWidth = Math.max(180, Math.min(480, sw + e2.clientX - sx));
    };
    const mu = () => {
      dragging = false;
      localStorage.setItem('sb-w', String(sbWidth));
      document.removeEventListener('mousemove', mm);
      document.removeEventListener('mouseup', mu);
    };
    document.addEventListener('mousemove', mm);
    document.addEventListener('mouseup', mu);
    e.preventDefault();
  }

  // ── Virtual scroll ────────────────────────────────────────────────────────
  const ROW_H = 28;
  let listEl = $state<HTMLElement | undefined>(undefined);
  let scrollTop = $state(0),
    clientH = $state(600);
  const vStart = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - 10));
  const vEnd = $derived(
    Math.min(app.filteredWords.length, Math.ceil((scrollTop + clientH) / ROW_H) + 10)
  );
  const topPad = $derived(vStart * ROW_H);
  const botPad = $derived(Math.max(0, (app.filteredWords.length - vEnd) * ROW_H));

  // Auto-scroll functionality removed

  // ── Keyboard nav ─────────────────────────────────────────────────────────
  function searchKeydown(e: KeyboardEvent) {
    if (app.searchMode === 'el') {
      if (e.key === 'Enter') {
        e.preventDefault();
        searchEnglishNow();
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (app.filteredWords[0]) selectWord(app.filteredWords[0].id);
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (app.filteredWords[0]) selectWord(app.filteredWords[0].id);
    }
  }
  function itemKeydown(e: KeyboardEvent, absIdx: number) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const nxt = absIdx + 1;
      const nxtWord = app.filteredWords[nxt];
      if (nxt < app.filteredWords.length && nxtWord) {
        selectWord(nxtWord.id);
      }
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (absIdx === 0) document.getElementById('wsearch')?.focus();
      else {
        const prevWord = app.filteredWords[absIdx - 1];
        if (prevWord) {
          selectWord(prevWord.id);
        }
      }
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      const cur = app.filteredWords[absIdx];
      if (cur) selectWord(cur.id);
    }
  }
  // Auto-scroll functionality removed - list stays in place

  // ── Filter helpers ────────────────────────────────────────────────────────
  const groups = $derived([...new Set(app.types.map((t) => t.group_ || 'Other'))].sort());

  function clearFilters() {
    app.searchQ = '';
    app.typeFilter = '';
    applyFilter();
  }
  function clearSearch() {
    if (app.searchMode === 'el') searchEnglishDebounced('');
    else {
      app.searchQ = '';
      applyFilter();
    }
  }

  // Status line below search field — plain derived string, not a derived function.
  const statusLine = $derived(
    (() => {
      if (app.tab !== 'words') return '';
      if (app.searchMode === 'el') {
        if (app.elSearching) return 'Searching…';
        if (app.elQuery.trim() && app.elResults.length > 0) {
          const n = app.elResults.length;
          return `${n}${n >= 300 ? '+' : ''} word${n !== 1 ? 's' : ''}`;
        }
        return '';
      }
      // L→E
      const f = app.filteredWords.length,
        t = app.words.length;
      const active = app.searchQ.trim() || app.typeFilter;
      if (active && f < t) return `${f} / ${t}`;
      return t > 0 ? t.toLocaleString() : '';
    })()
  );

  const hasQuery = $derived(app.searchMode === 'el' ? !!app.elQuery : !!app.searchQ);
  const searchVal = $derived(app.searchMode === 'el' ? app.elQuery : app.searchQ);
  const placeholder = $derived(
    app.tab !== 'words'
      ? 'Search events…'
      : app.searchMode === 'el'
        ? 'English keyword…'
        : 'Prefix… (* wildcard)'
  );

  function handleInput(v: string) {
    if (app.searchMode === 'el') searchEnglishDebounced(v);
    else {
      app.searchQ = v;
      applyFilter();
    }
  }

  // Single toggle: shows CURRENT mode
  const modeLabel = $derived(app.searchMode === 'le' ? 'L' : 'E');
  const modeTitle = $derived(
    app.searchMode === 'le' ? 'Loglan→English (current)' : 'English→Loglan (current)'
  );
  const elMode = $derived(app.searchMode === 'el');
  function toggleMode() {
    setSearchMode(app.searchMode === 'le' ? 'el' : 'le');
  }
</script>

<aside class="sb" style="width:{sbWidth}px">
  <!-- Filter area -->
  {#if app.tab === 'words' || app.tab === 'events'}
    <div class="sb-filter">
      <!-- Search row: input + mode toggle button -->
      <div class="search-row">
        <div class="search-wrap">
          <input
            id="wsearch"
            type="text"
            class="sb-fi"
            {placeholder}
            autocomplete="off"
            value={searchVal}
            oninput={(e) => handleInput((e.target as HTMLInputElement).value)}
            onkeydown={searchKeydown}
          />
          {#if hasQuery}
            <button class="fi-clr" onclick={clearSearch} title="Clear"
              ><Icon name="close" size={14} /></button
            >
          {/if}
        </div>

        <!-- Compact mode toggle, right of input, only for Words tab -->
        {#if app.tab === 'words'}
          <button class="mode-btn" class:el={elMode} title={modeTitle} onclick={toggleMode}
            >{modeLabel}</button
          >
        {/if}
      </div>

      <!-- E→L sub-filters: FTS vs LIKE vs Keywords — shown below search in EL mode -->
      {#if app.tab === 'words' && elMode}
        <div class="filter-row el-filters">
          <button
            class="el-filter-btn"
            class:el-filter-on={!app.prefs.elUseKeywords && !app.prefs.elUseLike}
            onclick={() => {
              setPref('elUseKeywords', false);
              setPref('elUseLike', false);
              if (app.elQuery) searchEnglishNow();
            }}
            title="Full-text search across all definition text (FTS5)"
          >
            FTS
          </button>
          <button
            class="el-filter-btn"
            class:el-filter-on={app.prefs.elUseLike}
            onclick={() => {
              setPref('elUseLike', true);
              setPref('elUseKeywords', false);
              if (app.elQuery) searchEnglishNow();
            }}
            title="Pattern matching with LIKE operator"
          >
            LIKE
          </button>
          <button
            class="el-filter-btn"
            class:el-filter-on={app.prefs.elUseKeywords}
            onclick={() => {
              setPref('elUseKeywords', true);
              setPref('elUseLike', false);
              if (app.elQuery) searchEnglishNow();
            }}
            title="Search only highlighted «keywords» in definitions"
          >
            Keywords
          </button>
        </div>
      {/if}

      <!-- Type dropdown — only in L→E words mode -->
      {#if app.tab === 'words' && app.searchMode === 'le'}
        <div class="filter-row">
          <select class="sb-sel" bind:value={app.typeFilter} onchange={() => applyFilter()}>
            <option value="">All types</option>
            {#each groups as g}
              <option value="__g__{g}">▶ {g}</option>
              {#each app.types.filter((t) => (t.group_ || 'Other') === g) as t}
                <option value={t.name}>&nbsp;&nbsp;{t.name}</option>
              {/each}
            {/each}
          </select>
          {#if app.typeFilter}
            <button class="clr-btn" onclick={clearFilters} title="Clear type filter"
              ><Icon name="close" size={14} /></button
            >
          {/if}
        </div>
      {/if}

      <!-- Status line: word count / result count — always outside the field -->
      {#if app.tab === 'words'}
        <div class="status-line">
          {#if statusLine}
            <span class="status-count">{statusLine}</span>
          {/if}
          {#if elMode && !app.elFtsReady && app.dbOpen}
            <span class="fts-warn" title="FTS index not ready — open Tools → Settings → Rebuild FTS"
              >⚠ LIKE mode</span
            >
          {/if}
          {#if app.typeFilter && app.searchMode === 'le'}
            <button class="clr-all" onclick={clearFilters}>clear all ×</button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- List — scrollbar on the LEFT via direction:rtl trick -->
  <div
    class="sb-list"
    id="sb-list"
    bind:this={listEl}
    bind:clientHeight={clientH}
    onscroll={(e) => (scrollTop = (e.currentTarget as HTMLElement).scrollTop)}
  >
    <!-- inner wrapper restores LTR for content -->
    <div class="sb-list-inner">
      {#if app.tab === 'words'}
        {#if app.searchMode === 'el'}
          <ELResults />
        {:else if app.filteredWords.length === 0}
          <div class="empty">No words found</div>
        {:else}
          <div style="height:{topPad}px;flex-shrink:0"></div>
          {#each app.filteredWords.slice(vStart, vEnd) as w, i (w.id)}
            <div
              class="si"
              style="height:{ROW_H}px"
              class:on={app.curWord?.id === w.id || app.loadingWordId === w.id}
              class:loading={app.loadingWordId === w.id}
              role="button"
              tabindex="0"
              onclick={(e) => {
                e.preventDefault();
                selectWord(w.id);
              }}
              onkeydown={(e) => itemKeydown(e, vStart + i)}
            >
              <span class="sn">{w.name}</span>
              {#if app.prefs.showTypeTag && w.type_name}
                <span class="st">{w.type_name}</span>
              {/if}
              {#if app.prefs.showDefCount && w.def_count > 0}
                <span class="sdc">{w.def_count}</span>
              {/if}
            </div>
          {/each}
          <div style="height:{botPad}px;flex-shrink:0"></div>
        {/if}
      {:else if app.tab === 'events'}
        {#each app.events as ev, i}
          <div
            class="si"
            style="height:{ROW_H}px"
            class:on={app.curEvent?.id === ev.id}
            role="button"
            tabindex="0"
            onclick={(e) => {
              e.preventDefault();
              selectEvent(ev.id);
            }}
            onkeydown={(e) => itemKeydown(e, i)}
          >
            <span class="sn">{ev.name}</span>
            {#if ev.annotation}<span class="st">{ev.annotation}</span>{/if}
          </div>
        {:else}
          <div class="empty">No events</div>
        {/each}
      {:else if app.tab === 'types' || app.tab === 'authors'}
        <div class="sb-redirect">
          <Icon name={app.tab} size={20} />
          <span>{app.tab === 'types' ? 'Types' : 'Authors'}</span>
        </div>
      {/if}
    </div>
    <!-- /sb-list-inner -->
  </div>

  <!-- Resize handle — far right, doesn't touch scrollbar -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="sb-resize"
    class:drag={dragging}
    onmousedown={dragStart}
    role="separator"
    aria-label="Resize sidebar"
    aria-orientation="vertical"
  ></div>
</aside>

<style>
  /* ── Shell ────────────────────────────────────────────────────────────────── */
  .sb {
    height: 100%;
    background: var(--sb-bg);
    border-right: 1px solid var(--sb-border);
    display: flex;
    flex-direction: column;
    position: relative;
    flex-shrink: 0;
    min-width: 180px;
    max-width: 480px;
  }
  @media (max-width: 640px) {
    .sb {
      width: 100% !important;
      min-width: 100%;
      max-width: 100%;
      border-right: none;
      height: 100%;
    }
  }

  /* Resize handle — right edge, thin, z above scrollbar */
  .sb-resize {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 3px;
    cursor: col-resize;
    z-index: 30;
    transition: background 140ms;
  }
  .sb-resize:hover,
  .drag {
    background: rgba(106, 170, 112, 0.45);
  }
  @media (max-width: 640px) {
    .sb-resize {
      display: none;
    }
  }

  /* ── Filter area ─────────────────────────────────────────────────────────── */
  .sb-filter {
    padding: 0.28rem 0.32rem 0.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
    flex-shrink: 0;
    background: var(--sb-bg); /* always opaque — list can't show through */
    border-bottom: 1px solid var(--sb-border);
    position: relative; /* stacking context above the list */
    z-index: 2;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }
  .sb-sel {
    flex: 1;
    min-width: 0;
    height: 28px;
    background: var(--inp-bg);
    border: 1px solid var(--inp-border);
    color: var(--sb-text);
    padding: 0 0.3rem;
    padding-right: 18px;
    border-radius: var(--r-md);
    font-family: inherit;
    font-size: 0.65rem;
    outline: none;
    -webkit-appearance: none;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='5'%3E%3Cpath d='M0 0l4 5 4-5z' fill='%23605848'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 6px center;
    background-size: 6px 4px;
  }
  .sb-sel:focus {
    border-color: var(--gold-d);
  }
  .sb-sel option {
    background: var(--surf2);
  }
  .clr-btn {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
    color: var(--text3);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    cursor: pointer;
    font-family: inherit;
    transition: all 140ms;
  }
  .clr-btn:hover {
    color: var(--red);
    border-color: var(--red-d);
    background: var(--red-g);
  }

  /* Search row: input + mode toggle */
  .search-row {
    display: flex;
    align-items: center;
    gap: 0.22rem;
  }
  .search-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
  }
  .sb-fi {
    width: 100%;
    height: 28px;
    background: var(--inp-bg);
    border: 1px solid var(--inp-border);
    color: var(--sb-text);
    padding: 0 1.6rem 0 0.42rem;
    border-radius: var(--r-md);
    font-family: inherit;
    font-size: 0.68rem;
    outline: none;
    -webkit-appearance: none;
    appearance: none;
  }
  .sb-fi:focus {
    border-color: var(--gold-d);
  }
  .sb-fi::placeholder {
    color: var(--sb-text2);
    font-size: 0.64rem;
  }
  @media (max-width: 640px) {
    .sb-fi {
      font-size: 1rem;
      height: 36px;
    }
    .sb-sel {
      font-size: 0.6rem;
      height: 36px;
    }
  }
  .fi-clr {
    position: absolute;
    right: 0.3rem;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--sb-text2);
    cursor: pointer;
    font-size: 0.7rem;
  }
  .fi-clr:hover {
    color: var(--text);
  }

  /* Mode toggle button — compact square right of input */
  .mode-btn {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border-radius: 4px;
    border: 1px solid var(--inp-border);
    background: var(--inp-bg);
    color: var(--sb-text2);
    font-family: inherit;
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: all 140ms;
    display: flex;
    align-items: center;
    justify-content: center;
    -webkit-tap-highlight-color: transparent;
  }
  .mode-btn:hover {
    background: var(--sb-hover);
    color: var(--sb-text);
    border-color: var(--gold-d);
  }
  /* When in E→L mode, button shows "L" and is tinted */
  .mode-btn.el {
    background: var(--gold-g);
    border-color: var(--gold-d);
    color: var(--gold);
  }
  @media (max-width: 640px) {
    .mode-btn {
      width: 36px;
      height: 36px;
      font-size: 1rem;
    }
  }

  /* E→L sub-filter row (FTS / Keywords) */
  .el-filters {
    gap: 0.25rem;
  }
  .el-filter-btn {
    flex: 1;
    height: 22px;
    border-radius: var(--r-sm);
    border: 1px solid var(--inp-border);
    background: var(--inp-bg);
    color: var(--sb-text2);
    font-family: inherit;
    font-size: 0.54rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: all 140ms;
    -webkit-tap-highlight-color: transparent;
    padding: 0 0.2rem;
  }
  .el-filter-btn:hover {
    border-color: var(--gold-d);
    color: var(--sb-text);
  }
  .el-filter-btn.el-filter-on {
    background: var(--gold-g);
    border-color: var(--gold-d);
    color: var(--gold);
  }
  @media (max-width: 640px) {
    .el-filter-btn {
      height: 28px;
      font-size: 0.62rem;
      padding: 0 0.3rem;
    }
  }

  /* Status line */
  .status-line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    height: 16px;
    padding: 0 0.1rem;
    flex-shrink: 0;
    /* already inside .sb-filter which is opaque */
  }
  .status-count {
    font-size: 0.58rem;
    color: var(--sb-text2);
    letter-spacing: 0.03em;
  }
  .fts-warn {
    font-size: 0.54rem;
    color: var(--red, var(--red));
    cursor: help;
  }
  .clr-all {
    margin-left: auto;
    font-size: 0.52rem;
    color: var(--text3);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: inherit;
  }
  .clr-all:hover {
    color: var(--red);
  }

  /* ── List ─────────────────────────────────────────────────────────────────── */
  /* Resize handle is 3px wide at right:0.
     Scrollbar sits just left of it: we reserve 8px on the right so they never overlap. */
  .sb-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
    scrollbar-width: thin;
    scrollbar-color: var(--sb-border) transparent;
    /* NO padding-right: items fill full width so highlight reaches edge */
    /* Prevent scroll on focus */
    scroll-behavior: auto;
  }
  .sb-list::-webkit-scrollbar {
    width: 5px;
  }
  .sb-list::-webkit-scrollbar-track {
    background: transparent;
  }
  .sb-list::-webkit-scrollbar-thumb {
    background: var(--sb-border);
    border-radius: 3px;
  }

  /* Inner wrapper: no extra padding needed now */
  .sb-list-inner {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  /* ── List items ──────────────────────────────────────────────────────────── */
  .si {
    display: flex;
    align-items: center;
    gap: 0.22rem;
    /* right: 6px clears 4px scrollbar + 2px visual gap */
    padding: 0 6px 0 6px;
    cursor: pointer;
    border-left: 2px solid transparent;
    transition:
      background 100ms,
      border-color 100ms;
    outline: none;
    flex-shrink: 0;
    -webkit-tap-highlight-color: transparent;
    touch-action: manipulation;
    /* Prevent scroll on focus */
    scroll-margin: 0;
    scroll-padding: 0;
  }
  .si:hover,
  .si:focus {
    background: var(--sb-hover);
  }
  .si.on {
    background: var(--gold-g);
    border-left-color: var(--gold);
  }
  .si.loading {
    background: var(--sb-hover);
    border-left-color: var(--gold);
    animation: si-pulse 0.9s ease-in-out infinite alternate;
    pointer-events: none;
  }
  @keyframes si-pulse {
    from {
      opacity: 1;
    }
    to {
      opacity: 0.5;
    }
  }
  .sn {
    font-size: 0.73rem;
    font-weight: 500;
    color: var(--sb-text);
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .si.on .sn {
    color: var(--gold);
  }
  .st {
    font-size: 0.5rem;
    color: var(--sb-text2);
    background: rgba(128, 128, 128, 0.1);
    padding: 2px 5px;
    border-radius: var(--r-sm);
    flex-shrink: 0;
    max-width: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }
  .sdc {
    font-size: 0.5rem;
    color: var(--blue);
    background: rgba(106, 154, 200, 0.1);
    padding: 2px 5px;
    border-radius: var(--r-sm);
    flex-shrink: 0;
    min-width: 16px;
    text-align: center;
  }
  @media (max-width: 640px) {
    .si {
      height: 44px !important;
    }
    .sn {
      font-size: 1rem;
    }
    .st {
      font-size: 0.65rem;
      padding: 4px 8px;
      border-radius: var(--r-md);
    }
    .sdc {
      font-size: 0.65rem;
      padding: 4px 8px;
      border-radius: var(--r-md);
      min-width: 20px;
    }
  }

  .empty {
    padding: 1.2rem 0.5rem;
    font-size: 0.68rem;
    color: var(--sb-text2);
    text-align: center;
  }
  .sb-redirect {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.65rem;
    color: var(--sb-text2);
    padding: 1rem;
    opacity: 0.5;
  }
</style>
