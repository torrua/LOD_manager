<script lang="ts">
  import { onMount } from 'svelte';
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { platform } from '@tauri-apps/plugin-os';
  import { appDataDir } from '@tauri-apps/api/path';
  import {
    app,
    toggleTheme,
    toggleReadonly,
    toast,
    goBack,
    goForward,
    canGoBack,
    canGoForward,
    getLastDbPath,
    openDb,
    createDb,
    activeEvent,
  } from './lib/store.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import WordDetail from './lib/components/WordDetail.svelte';
  import WordForm from './lib/components/WordForm.svelte';
  import EventDetail from './lib/components/EventDetail.svelte';
  import EventForm from './lib/components/EventForm.svelte';
  import TypesPanel from './lib/components/TypesPanel.svelte';
  import AuthorsPanel from './lib/components/AuthorsPanel.svelte';
  import ToolsDrawer from './lib/components/ToolsDrawer.svelte';
  import Toast from './lib/components/Toast.svelte';

  onMount(() => {
    document.documentElement.dataset.theme = app.theme;

    // Auto-create database on Android if no database exists
    const currentPlatform = platform();
    const last = getLastDbPath();

    if (currentPlatform === 'android' && !last) {
      appDataDir()
        .then((appDataPath) => {
          const dbPath = `${appDataPath}lod.db`;
          createDb(dbPath).catch((e) => {
            console.error('Failed to auto-create Android database:', e);
            // Fall back to normal behavior
          });
        })
        .catch((e) => {
          console.error('Failed to get app data directory:', e);
        });
    } else if (last) {
      openDb(last).catch(() => {});
    }

    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 'f') {
        e.preventDefault();
        document.getElementById('wsearch')?.focus();
      }
      if (e.ctrlKey && e.key === 'n' && app.dbOpen && !app.readonly) {
        e.preventDefault();
        handleNew();
      }
      if (e.key === 'Escape') {
        if (app.toolsOpen) {
          app.toolsOpen = false;
          return;
        }
        if (app.editing) {
          app.editing = false;
          app.panel = app.curWord ? 'word' : app.curEvent ? 'event' : 'welcome';
        }
      }
      if (e.altKey && e.key === 'ArrowLeft') {
        e.preventDefault();
        goBack();
      }
      if (e.altKey && e.key === 'ArrowRight') {
        e.preventDefault();
        goForward();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  });

  $effect(() => {
    if (app.readonly) document.documentElement.setAttribute('data-ro', '1');
    else document.documentElement.removeAttribute('data-ro');
  });

  // ── compact bottom-bar new-item sheet ─────────────────────────────
  let newSheetOpen = $state(false);

  const ALL_ITEMS = [
    {
      tab: 'words' as const,
      icon: '📝',
      label: 'New Word',
      action: () => {
        app.tab = 'words';
        app.panel = 'word-form';
        app.curWord = null;
        app.editing = true;
        app.mobileShowList = false;
      },
    },
    {
      tab: 'events' as const,
      icon: '📅',
      label: 'New Event',
      action: () => {
        app.tab = 'events';
        app.panel = 'event-form';
        app.curEvent = null;
        app.editing = true;
        app.mobileShowList = false;
      },
    },
    {
      tab: 'types' as const,
      icon: '🏷',
      label: 'New Type',
      action: () => {
        app.tab = 'types';
        app.panel = 'types';
        app.mobileShowList = false;
        app.newSignal++;
      },
    },
    {
      tab: 'authors' as const,
      icon: '👤',
      label: 'New Author',
      action: () => {
        app.tab = 'authors';
        app.panel = 'authors';
        app.mobileShowList = false;
        app.newSignal++;
      },
    },
  ];
  const sheetItems = $derived(app.readonly ? [] : ALL_ITEMS);

  function handleNew() {
    if (app.tab === 'words') {
      app.panel = 'word-form';
      app.curWord = null;
      app.editing = true;
      app.mobileShowList = false;
    }
    if (app.tab === 'events') {
      app.panel = 'event-form';
      app.curEvent = null;
      app.editing = true;
      app.mobileShowList = false;
    }
    if (app.tab === 'types' || app.tab === 'authors') {
      app.newSignal++;
    }
  }
  function newLabel() {
    if (app.tab === 'words') return '＋ Word';
    if (app.tab === 'events') return '＋ Event';
    if (app.tab === 'types') return '＋ Type';
    if (app.tab === 'authors') return '＋ Author';
    return '';
  }
  function handleLogoClick() {
    app.editing = false;
    app.mobileShowList = true;
    app.tab = 'words';
    app.panel = app.curWord ? 'word' : 'welcome';
    newSheetOpen = false;
  }
  const canNew = $derived(
    app.dbOpen &&
      !app.readonly &&
      (app.tab === 'words' || app.tab === 'events' || app.tab === 'types' || app.tab === 'authors')
  );

  // Close sheet on Escape
  $effect(() => {
    const close = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && newSheetOpen) {
        newSheetOpen = false;
        e.stopPropagation();
      }
    };
    document.addEventListener('keydown', close, true);
    return () => document.removeEventListener('keydown', close, true);
  });
</script>

<div class="app" data-theme={app.theme}>
  <header class="top-bar">
    <div class="tbl">
      {#if app.dbOpen && !app.mobileShowList}
        <button
          class="btn btn-icon btn-ghost mob-only"
          onclick={() =>
            app.tab === 'types' || app.tab === 'authors' ? goBack() : (app.mobileShowList = true)}
          title="Back">←</button
        >
      {/if}
      <button class="logo" onclick={handleLogoClick} aria-label="Home">LOD</button>
      {#if app.dbOpen && activeEvent()}
        {@const ev = activeEvent()}
        <span class="ev-badge" title="Filtered to: {ev?.name}">{ev?.annotation || ev?.name}</span>
      {/if}
    </div>
    <div class="tbr">
      {#if app.dbOpen}
        <button
          class="btn btn-icon btn-ghost"
          onclick={goBack}
          disabled={!canGoBack()}
          title="Back (Alt+←)">←</button
        >
        <button
          class="btn btn-icon btn-ghost"
          onclick={goForward}
          disabled={!canGoForward()}
          title="Forward (Alt+→)">→</button
        >
        {#if canNew}
          <!-- desktop: direct new; compact: opens bottom sheet -->
          <button class="btn btn-sm btn-g hide-compact" onclick={handleNew}>{newLabel()}</button>
        {/if}
        <div class="sep hide-compact"></div>
      {/if}
      <button
        class="btn btn-icon btn-ghost hide-compact"
        class:btn-active={app.readonly}
        onclick={toggleReadonly}
        title={app.readonly ? 'Exit read-only' : 'Read-only'}
      >
        {app.readonly ? '✏' : '👁'}
      </button>
      <button class="btn btn-icon btn-ghost hide-compact" onclick={toggleTheme} title="Toggle theme"
        >◑</button
      >
      <button
        class="btn btn-icon btn-ghost hide-compact"
        class:btn-active={app.toolsOpen}
        onclick={() => (app.toolsOpen = !app.toolsOpen)}
        title="Tools">⚙</button
      >
    </div>
  </header>

  {#if !app.dbOpen}
    <div class="no-db">
      <div class="no-db-inner">
        <div class="no-db-icon">🕮</div>
        <h1>Loglan Online Dictionary</h1>
        <p>Open an existing database or create a new one.</p>
        <div class="no-db-btns">
          <button
            class="btn btn-au btn-lg"
            onclick={async () => {
              const p = await openDialog({
                title: 'Open LOD Database',
                filters: [{ name: 'SQLite', extensions: ['db', 'sqlite', 'sqlite3'] }],
              });
              if (p) await openDb(p as string).catch((e) => toast(String(e), 'err'));
            }}>Open DB</button
          >
          <button
            class="btn btn-g btn-lg"
            onclick={async () => {
              const p = await saveDialog({
                title: 'Create New Database',
                defaultPath: 'loglan.db',
                filters: [{ name: 'SQLite', extensions: ['db'] }],
              });
              if (p) await createDb(p).catch((e) => toast(String(e), 'err'));
            }}>New DB</button
          >
        </div>
      </div>
    </div>
  {:else}
    <!-- workspace takes remaining height minus bottom-bar on compact -->
    <div class="workspace">
      <div
        class="sb-wrap"
        class:mob-hidden={!app.mobileShowList ||
          app.tab === 'types' ||
          app.tab === 'authors' ||
          app.panel === 'word-form' ||
          app.panel === 'event-form'}
      >
        <Sidebar />
      </div>
      <main
        class="main-content"
        class:mob-hidden={app.mobileShowList &&
          app.panel !== 'welcome' &&
          app.panel !== 'word-form' &&
          app.panel !== 'event-form' &&
          app.tab !== 'types' &&
          app.tab !== 'authors'}
      >
        {#if app.panel === 'welcome'}
          <div class="empty">
            <div class="empty-icon">🕮</div>
            {#if app.tab === 'events'}
              <h3>Select an event</h3>
              <p>{app.events.length} events</p>
            {:else if app.tab === 'types'}
              <h3>Types</h3>
            {:else if app.tab === 'authors'}<h3>Authors</h3>
            {:else}<h3>Select a word</h3>
              <p>{app.wordCount.toLocaleString()} words · {app.events.length} events</p>
            {/if}
          </div>
        {:else if app.loadingWordId !== null && !app.curWord}
          <!-- First-open skeleton: shown while very first word loads -->
          <div class="wd-skeleton" aria-busy="true" aria-label="Loading…">
            <div class="sk-line sk-title"></div>
            <div class="sk-line sk-meta"></div>
            <div class="sk-line"></div>
            <div class="sk-line sk-short"></div>
            <div class="sk-line"></div>
            <div class="sk-line sk-short"></div>
          </div>
        {:else if app.panel === 'word' && app.curWord}
          <WordDetail word={app.curWord} loading={app.loadingWordId !== null} />
        {:else if app.panel === 'word-form'}
          <WordForm />
        {:else if app.panel === 'event' && app.curEvent}
          <EventDetail event={app.curEvent} />
        {:else if app.panel === 'event-form'}
          <EventForm />
        {:else if app.panel === 'types'}
          <TypesPanel />
        {:else if app.panel === 'authors'}
          <AuthorsPanel />
        {/if}
      </main>
    </div>

    <!-- ── COMPACT BOTTOM BAR — fixed at bottom of screen ── -->
    <div class="bottom-bar show-compact">
      <div class="bb-left">
        {#if canNew}
          <button class="btn btn-sm btn-g" onclick={() => (newSheetOpen = true)}
            >{newLabel()}</button
          >
        {/if}
      </div>
      <div class="bb-right">
        <button
          class="btn btn-icon btn-ghost"
          class:btn-active={app.readonly}
          onclick={toggleReadonly}
          title={app.readonly ? 'Exit read-only' : 'Read-only'}
        >
          {app.readonly ? '✏' : '👁'}
        </button>
        <button class="btn btn-icon btn-ghost" onclick={toggleTheme}>◑</button>
        <button
          class="btn btn-icon btn-ghost"
          class:btn-active={app.toolsOpen}
          onclick={() => (app.toolsOpen = !app.toolsOpen)}>⚙</button
        >
      </div>
    </div>
  {/if}

  <!-- ── NEW ITEM BOTTOM SHEET (compact mode) ── -->
  {#if newSheetOpen}
    <button class="sheet-backdrop" onclick={() => (newSheetOpen = false)} aria-label="Close"
    ></button>
    <div class="new-sheet">
      <div class="sheet-handle"></div>
      <div class="sheet-title">Create new…</div>
      {#each sheetItems as item}
        <button
          class="sheet-item"
          onclick={() => {
            newSheetOpen = false;
            item.action();
          }}
        >
          <span class="sheet-item-icon">{item.icon}</span>
          <span class="sheet-item-label">{item.label}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if app.toolsOpen}<ToolsDrawer />{/if}
  <Toast />
</div>

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  :global(html, body) {
    height: 100%;
    overflow: hidden;
    font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
  }
  :global(::-webkit-scrollbar) {
    width: 4px;
    height: 4px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: var(--border2);
    border-radius: 2px;
  }

  :global([data-theme='dark']) {
    --bg: #0d0c09;
    --surf: #141210;
    --surf2: #1a1815;
    --surf3: #201e1a;
    --border: #2a2620;
    --border2: #38342c;
    --border3: #4a4438;
    --text: #ddd0b8;
    --text2: #8a7e6c;
    --text3: #504838;
    --gold: #c8a454;
    --gold-d: #7a6030;
    --gold-g: rgba(200, 164, 84, 0.1);
    --green: #6aaa70;
    --green-d: #2a5a30;
    --green-g: rgba(106, 170, 112, 0.1);
    --red: #c07060;
    --red-d: #6a2818;
    --red-g: rgba(192, 112, 96, 0.1);
    --blue: #6a9ac8;
    --teal: #5ab8b0;
    --teal-d: #1a5850;
    --teal-g: rgba(90, 184, 176, 0.12);
    --sb-bg: #090806;
    --sb-text: #c8bfa8;
    --sb-text2: #605848;
    --sb-border: rgba(255, 255, 255, 0.05);
    --sb-hover: rgba(255, 255, 255, 0.04);
    --inp-bg: #1a1815;
    --inp-border: #38342c;
    --shd: rgba(0, 0, 0, 0.45);
    --shd-lg: rgba(0, 0, 0, 0.72);
    --overlay: rgba(0, 0, 0, 0.65);
    --bb-h: 48px;
    /* ─ Design system ─ */
    --sp-1: 4px;
    --sp-2: 8px;
    --sp-3: 12px;
    --sp-4: 16px;
    --sp-5: 24px;
    --r-sm: 3px;
    --r-md: 4px;
    --r-lg: 6px;
    --r-xl: 8px;
    --fs-label: 0.52rem;
    --fs-xs: 0.65rem;
    --fs-sm: 0.72rem;
    --fs-base: 0.78rem;
    --fs-md: 0.82rem;
    --fs-lg: 1rem;
    --fs-xl: 1.4rem;
    --red: #c44444;
    --green: #5a8;
  }
  :global([data-theme='light']) {
    --bg: #f0ebe0;
    --surf: #faf6ee;
    --surf2: #f3ede2;
    --surf3: #ece5d8;
    --border: #d8d0c0;
    --border2: #c8bfad;
    --border3: #b8ae9a;
    --text: #1e1608;
    --text2: #6a5c48;
    --text3: #9a8c78;
    --gold: #7a5418;
    --gold-d: #c8a050;
    --gold-g: rgba(122, 84, 24, 0.08);
    --green: #246028;
    --green-d: #7ab87a;
    --green-g: rgba(36, 96, 40, 0.08);
    --red: #8a2418;
    --red-d: #d07060;
    --red-g: rgba(138, 36, 24, 0.08);
    --blue: #1e4a7a;
    --teal: #1a6860;
    --teal-d: #5ab8b0;
    --teal-g: rgba(26, 104, 96, 0.1);
    --sb-bg: #e8e0d0;
    --sb-text: #2a2018;
    --sb-text2: #7a6c58;
    --sb-border: rgba(0, 0, 0, 0.1);
    --sb-hover: rgba(0, 0, 0, 0.04);
    --inp-bg: #f3ede2;
    --inp-border: #c8bfad;
    --shd: rgba(0, 0, 0, 0.1);
    --shd-lg: rgba(0, 0, 0, 0.22);
    --overlay: rgba(0, 0, 0, 0.52);
    --bb-h: 48px;
    /* ─ Design system (same scale, same vars) ─ */
    --sp-1: 4px;
    --sp-2: 8px;
    --sp-3: 12px;
    --sp-4: 16px;
    --sp-5: 24px;
    --r-sm: 3px;
    --r-md: 4px;
    --r-lg: 6px;
    --r-xl: 8px;
    --fs-label: 0.52rem;
    --fs-xs: 0.65rem;
    --fs-sm: 0.72rem;
    --fs-base: 0.78rem;
    --fs-md: 0.82rem;
    --fs-lg: 1rem;
    --fs-xl: 1.4rem;
    --red: #c44444;
    --green: #5a8;
  }

  /* ── Buttons ── */
  :global(.btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.22rem;
    height: 28px;
    padding: 0 0.6rem;
    font-family: inherit;
    font-size: 0.7rem;
    font-weight: 500;
    line-height: 1;
    color: var(--text2);
    background: var(--surf2);
    border: 1px solid var(--border2);
    border-radius: var(--r-md);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 120ms,
      border-color 120ms,
      color 120ms,
      transform 80ms;
    -webkit-tap-highlight-color: transparent;
    touch-action: manipulation;
  }
  :global(.btn:hover:not(:disabled)) {
    color: var(--text);
    background: var(--surf3);
    border-color: var(--border3);
  }
  :global(.btn:active:not(:disabled)) {
    transform: scale(0.97);
  }
  :global(.btn:disabled) {
    opacity: 0.35;
    cursor: default;
  }
  :global(.btn-g) {
    color: var(--green);
    background: var(--green-g);
    border-color: var(--green-d);
  }
  :global(.btn-au) {
    color: var(--gold);
    background: var(--gold-g);
    border-color: var(--gold-d);
  }
  :global(.btn-r) {
    color: var(--red);
    background: var(--red-g);
    border-color: var(--red-d);
  }
  :global(.btn-ghost) {
    background: transparent;
    border-color: transparent;
    color: var(--text3);
  }
  :global(.btn-ghost:hover:not(:disabled)) {
    background: var(--surf2);
    border-color: var(--border);
    color: var(--text2);
  }
  :global(.btn-active) {
    color: var(--gold) !important;
  }
  /* icon/size variants — all use the same base btn */
  :global(.btn-icon) {
    width: 28px;
    height: 28px;
    padding: 0;
    font-size: 1rem;
  }
  /* btn-icon-sm: 26×26px square for compact toolbars/inline lists */
  :global(.btn-icon-sm) {
    width: 26px;
    height: 26px;
    padding: 0;
    font-size: 0.85rem;
    flex-shrink: 0;
  }
  :global(.btn-sm) {
    height: 26px;
    padding: 0 0.48rem;
    font-size: var(--fs-xs);
  }
  :global(.btn-lg) {
    height: 36px;
    padding: 0 1.1rem;
    font-size: var(--fs-md);
  }

  /* ── Forms ── */
  :global(.fi, .fsel, .fta) {
    background: var(--inp-bg);
    border: 1px solid var(--inp-border);
    color: var(--text);
    padding: 0.28rem 0.48rem;
    border-radius: var(--r-md);
    font-family: inherit;
    font-size: var(--fs-base);
    outline: none;
    -webkit-appearance: none;
    appearance: none;
    transition: border-color 140ms;
    width: 100%;
    height: 28px;
  }
  :global(.fta) {
    height: auto;
    min-height: 64px;
    resize: vertical;
  }
  :global(.fi:focus, .fsel:focus, .fta:focus) {
    border-color: var(--gold-d);
  }
  :global(.fsel) {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%238a7e6c'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    background-size: 8px 5px;
    padding-right: 24px;
  }
  :global(.fsel option) {
    background: var(--surf2);
  }
  :global(.fg) {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    min-width: 0;
  }
  :global(.fg label) {
    font-size: 0.52rem;
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.09em;
  }
  :global(.form-row) {
    display: grid;
    gap: 0.5rem;
    margin-bottom: 0.44rem;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  }
  :global(.form-actions) {
    display: flex;
    flex-wrap: wrap;
    gap: 0.38rem;
    margin-top: 0.7rem;
    padding-top: 0.58rem;
    border-top: 1px solid var(--border);
  }

  /* ── Shared tokens ── */
  :global(.sec) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 28px;
    padding: 0.3rem 0;
    border-bottom: 1px solid var(--border);
  }
  :global(.sec-title) {
    font-size: 0.62rem;
    font-weight: 600;
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  :global(.data-table) {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.77rem;
  }
  :global(.data-table th) {
    text-align: left;
    padding: 0.28rem 0.52rem;
    font-size: 0.52rem;
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    border-bottom: 1px solid var(--border);
  }
  :global(.data-table td) {
    padding: 0.3rem 0.52rem;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }
  :global(.data-table tr:last-child td) {
    border-bottom: none;
  }
  :global(.data-table tr:hover td) {
    background: var(--surf2);
  }
  :global(.td-name) {
    color: var(--gold);
    font-weight: 600;
    font-size: 0.78rem;
  }
  :global(.td-sub) {
    color: var(--text2);
    font-size: 0.7rem;
  }
  :global(.row-acts) {
    opacity: 0;
    display: flex;
    gap: 0.15rem;
    justify-content: flex-end;
    transition: opacity 140ms;
  }
  :global(.data-table tr:hover .row-acts) {
    opacity: 1;
  }
  :global(.badge) {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    font-size: var(--fs-label);
  }
  :global(.bd-afx) {
    background: var(--green-g);
    border: 1px solid var(--green-d);
    color: var(--green);
    cursor: pointer;
  }
  :global(.bd-spell) {
    background: var(--teal-g);
    border: 1px solid var(--teal-d);
    color: var(--teal);
  }
  /* ─ Skeleton loader ────────────────────────────────────────────────────── */
  .wd-skeleton {
    padding: 1.1rem 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    flex: 1;
  }
  .sk-line {
    height: 13px;
    border-radius: 5px;
    background: linear-gradient(
      90deg,
      var(--surf2) 0%,
      var(--border) 40%,
      var(--surf2) 80%
    );
    background-size: 250% 100%;
    animation: sk-sweep 1.4s ease-in-out infinite;
  }
  .sk-title { height: 21px; width: 52%; }
  .sk-meta  { height: 10px; width: 72%; }
  .sk-short { width: 62%; }
  @keyframes sk-sweep {
    0%   { background-position: 200% 0; }
    100% { background-position: -100% 0; }
  }

  /* ─ Meta chip labels (used in WordDetail, EventDetail) ─ */
  :global(.mc-lbl) {
    font-size: var(--fs-label);
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    font-weight: 600;
    line-height: 1.2;
  }
  :global(.mc-val) {
    font-size: var(--fs-sm);
    color: var(--text);
    font-weight: 600;
    line-height: 1.3;
  }
  /* ─ Section divider row ─ */
  :global(.sec-row) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0 0.35rem; /* symmetric — button never touches border */
    border-bottom: 1px solid var(--border);
    margin-top: 0.9rem;
  }
  /* ─ Inline form panel ─ */
  :global(.inline-form) {
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    padding: 0.6rem 0.75rem;
    margin: 0.35rem 0;
  }
  :global(.if-title) {
    font-size: var(--fs-xs);
    font-weight: 700;
    color: var(--text2);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 0.5rem;
  }
  :global(.kw) {
    color: var(--teal);
    font-style: italic;
  }
  :global(.xref) {
    color: var(--blue);
    text-decoration: underline;
    text-decoration-style: dotted;
    cursor: pointer;
  }
  :global(.xref:hover) {
    color: var(--gold);
  }
  :global(.empty) {
    text-align: center;
    padding: 4rem 2rem;
    color: var(--text3);
  }
  :global(.empty-icon) {
    font-size: 2rem;
    margin-bottom: 0.6rem;
  }
  :global(.empty h3) {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text2);
    margin-bottom: 0.25rem;
  }
  :global(.empty p) {
    font-size: 0.72rem;
  }
  :global(.modal-overlay) {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
  }
  :global(.modal) {
    background: var(--surf);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 1.3rem 1.5rem;
    width: 100%;
    max-width: 420px;
    box-shadow: 0 8px 32px var(--shd-lg);
  }
  :global(.modal-title) {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text);
    margin-bottom: 0.4rem;
  }
  :global(.modal-body) {
    font-size: 0.75rem;
    color: var(--text2);
    margin-bottom: 0.85rem;
    line-height: 1.6;
  }
  :global(.modal-actions) {
    display: flex;
    gap: 0.38rem;
    justify-content: flex-end;
    flex-wrap: wrap;
  }
  :global([data-ro] .edit-only) {
    display: none !important;
  }

  /* ── Shell ── */
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    line-height: 1.6;
    overflow: hidden;
  }

  /* compact = window width < 520px */
  @media (max-width: 520px) {
    .hide-compact {
      display: none !important;
    }
    .show-compact {
      display: flex !important;
    }
    /* workspace shrinks to leave room for fixed bottom bar */
    .workspace {
      margin-bottom: var(--bb-h, 48px);
    }
  }
  @media (min-width: 521px) {
    .show-compact {
      display: none !important;
    }
  }

  .top-bar {
    background: var(--surf);
    border-bottom: 1px solid var(--border);
    /* padding-top accounts for Android status bar via safe-area-inset-top.
       viewport-fit=cover must be set in index.html for env() to work. */
    padding: env(safe-area-inset-top, 0px) 0.75rem 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: calc(42px + env(safe-area-inset-top, 0px));
    flex-shrink: 0;
    gap: 0.4rem;
  }
  .tbl {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    flex: 1;
    overflow: hidden;
  }
  .tbr {
    display: flex;
    align-items: center;
    gap: 0.1rem;
    flex-shrink: 0;
  }
  .ev-badge {
    font-size: 0.5rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 2px 5px;
    border-radius: var(--r-sm);
    background: rgba(128, 128, 128, 0.1);
    color: var(--text2);
    flex-shrink: 0;
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: default;
    font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
  }
  .logo {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--gold);
    cursor: pointer;
    letter-spacing: 0.04em;
    flex-shrink: 0;
    user-select: none;
    background: none;
    border: none;
    padding: 0;
    font-family: inherit;
    height: auto;
    min-height: unset;
    white-space: nowrap;
  }
  .logo:hover {
    color: var(--text);
  }
  .sep {
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 0.08rem;
    flex-shrink: 0;
  }
  .mob-only {
    display: none;
  }
  @media (max-width: 640px) {
    .mob-only {
      display: inline-flex;
    }
  }

  /* ── Fixed bottom bar (compact only) ── */
  .bottom-bar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    height: var(--bb-h, 48px);
    background: var(--surf);
    border-top: 1px solid var(--border);
    padding: 0 0.6rem;
    display: none; /* shown via show-compact */
    align-items: center;
    justify-content: space-between;
    z-index: 50;
    box-shadow: 0 -2px 12px var(--shd);
  }
  .bb-left {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .bb-right {
    display: flex;
    align-items: center;
    gap: 0.05rem;
  }

  .no-db {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
  }
  .no-db-inner {
    text-align: center;
    max-width: 380px;
  }
  .no-db-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }
  .no-db-inner h1 {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text);
    margin-bottom: 0.5rem;
  }
  .no-db-inner p {
    font-size: 0.75rem;
    color: var(--text2);
    margin-bottom: 1.4rem;
    line-height: 1.65;
  }
  .no-db-btns {
    display: flex;
    gap: 0.5rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  .workspace {
    flex: 1;
    display: flex;
    overflow: hidden;
    min-height: 0;
  }
  .sb-wrap {
    display: flex;
    flex-shrink: 0;
  }
  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem 1.5rem;
    min-width: 0;
  }

  @media (max-width: 640px) {
    .workspace {
      position: relative;
    }
    .sb-wrap {
      position: absolute;
      inset: 0;
      z-index: 1;
      transition: transform 200ms ease;
    }
    .sb-wrap.mob-hidden {
      transform: translateX(-100%);
      pointer-events: none;
    }
    .main-content {
      padding: 0.75rem 0.9rem;
    }
    .main-content.mob-hidden {
      display: none;
    }
  }

  /* ── New-item bottom sheet ── */
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: 200;
    animation: fade-in 150ms ease;
  }
  @keyframes fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .new-sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 201;
    background: var(--surf);
    border-top: 1px solid var(--border);
    border-radius: 16px 16px 0 0;
    padding: 0.6rem 0.75rem calc(0.75rem + env(safe-area-inset-bottom));
    box-shadow: 0 -4px 24px var(--shd-lg);
    animation: slide-up 220ms ease;
    min-height: 50vh; /* issue: stable height, no jumping between items */
    display: flex;
    flex-direction: column;
  }
  @keyframes slide-up {
    from {
      transform: translateY(100%);
    }
    to {
      transform: translateY(0);
    }
  }
  .sheet-handle {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background: var(--border2);
    margin: 0 auto 0.7rem;
  }
  .sheet-title {
    font-size: 0.62rem;
    font-weight: 700;
    color: var(--text3);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.5rem;
  }
  .sheet-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.65rem 0.75rem;
    border-radius: var(--r-lg);
    cursor: pointer;
    background: transparent;
    border: 1px solid transparent;
    font-family: inherit;
    font-size: 0.82rem;
    color: var(--text2);
    transition: all 140ms;
    text-align: left;
    margin-bottom: 0.18rem;
  }
  .sheet-item:hover {
    background: var(--surf2);
    border-color: var(--border);
    color: var(--text);
  }
  .sheet-item-icon {
    font-size: 1rem;
    width: 24px;
    text-align: center;
    flex-shrink: 0;
  }
  .sheet-item-label {
    font-size: 0.78rem;
    font-weight: 500;
  }
</style>
