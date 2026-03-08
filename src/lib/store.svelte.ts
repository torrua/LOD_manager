import { invoke } from '@tauri-apps/api/core';
import type {
  WordListItem,
  WordDetail,
  EventItem,
  TypeItem,
  AuthorItem,
  AppInfo,
  DbStats,
  ELResult,
  SearchMode,
  Tab,
} from '../types';

// ─── Preferences ─────────────────────────────────────────────────────────────
function loadPrefs() {
  try {
    return JSON.parse(localStorage.getItem('lod-prefs') || '{}');
  } catch {
    return {};
  }
}
function savePrefs() {
  localStorage.setItem('lod-prefs', JSON.stringify(app.prefs));
}

const DEFAULT_META = ['type', 'source', 'year', 'rank', 'match', 'event'];

export const app = $state({
  dbOpen: false,
  dbPath: '',
  wordCount: 0,
  theme: (localStorage.getItem('lod-theme') || 'dark') as 'dark' | 'light',
  readonly: localStorage.getItem('lod-ro') === '1',
  tab: 'words' as Tab,
  words: [] as WordListItem[],
  filteredWords: [] as WordListItem[],
  searchQ: '',
  typeFilter: '',
  curWord: null as WordDetail | null,
  curEvent: null as EventItem | null,
  types: [] as TypeItem[],
  authors: [] as AuthorItem[],
  events: [] as EventItem[],
  panel: 'welcome' as string,
  editing: false,
  history: [] as Array<{ tab: Tab; id: number }>,
  historyIdx: -1,
  toolsOpen: false,
  toolsTab: 'database' as 'import' | 'export' | 'database' | 'settings',
  newSignal: 0,
  mobileShowList: true,
  toast: null as { msg: string; kind: 'ok' | 'err' | 'info' } | null,
  searchMode: 'le' as SearchMode, // L→E or E→L
  elQuery: '',
  elResults: [] as ELResult[],
  elSearching: false,
  elFtsReady: false, // FTS index populated
  toastTimer: 0,
  dbStats: null as DbStats | null,
  prefs: {
    showTypeTag: (loadPrefs().showTypeTag ?? true) as boolean,
    showDefCount: (loadPrefs().showDefCount ?? false) as boolean,
    visibleMeta: (loadPrefs().visibleMeta ?? [...DEFAULT_META]) as string[],
    ...loadPrefs(),
    elShowSnippet: (loadPrefs().elShowSnippet ?? true) as boolean,
    elShowGrammar: (loadPrefs().elShowGrammar ?? true) as boolean,
    elShowType: (loadPrefs().elShowType ?? true) as boolean,
    elShowCount: (loadPrefs().elShowCount ?? true) as boolean,
    elUseLike: (loadPrefs().elUseLike ?? false) as boolean,
    elShowDetails: (loadPrefs().elShowDetails ?? true) as boolean,
    eventFilter: (loadPrefs().eventFilter ?? null) as number | null,
    showTooltips: (loadPrefs().showTooltips ?? true) as boolean,
  } as {
    showTypeTag: boolean;
    showDefCount: boolean;
    visibleMeta: string[];
    elShowSnippet: boolean;
    elShowGrammar: boolean;
    elShowType: boolean;
    elShowCount: boolean;
    elUseLike: boolean;
    elShowDetails: boolean;
    eventFilter: number | null;
    showTooltips: boolean;
  },
});

export function setPref<K extends keyof typeof app.prefs>(k: K, v: (typeof app.prefs)[K]) {
  (app.prefs as any)[k] = v;
  savePrefs();
}
export function toggleMetaField(field: string) {
  const idx = app.prefs.visibleMeta.indexOf(field);
  if (idx >= 0) app.prefs.visibleMeta = app.prefs.visibleMeta.filter((f) => f !== field);
  else app.prefs.visibleMeta = [...app.prefs.visibleMeta, field];
  savePrefs();
}

export function toggleTheme() {
  app.theme = app.theme === 'dark' ? 'light' : 'dark';
  localStorage.setItem('lod-theme', app.theme);
  document.documentElement.dataset.theme = app.theme;
}
export function toggleReadonly() {
  app.readonly = !app.readonly;
  localStorage.setItem('lod-ro', app.readonly ? '1' : '0');
}
export function toast(msg: string, kind: 'ok' | 'err' | 'info' = 'ok') {
  clearTimeout(app.toastTimer);
  app.toast = { msg, kind };
  app.toastTimer = setTimeout(() => {
    app.toast = null;
  }, 2800) as unknown as number;
}

export async function openDb(path: string) {
  const info: AppInfo = await invoke('open_database', { path });
  app.dbOpen = true;
  app.dbPath = info.db_path;
  localStorage.setItem('lod-last-db', path);
  await loadAll();
  app.panel = 'welcome';
  app.toolsOpen = false;
  toast(`Opened — ${app.words.length.toLocaleString()} words`, 'ok');
  checkFts().catch(() => {});
  loadDbStats().catch(() => {});
}
export async function createDb(path: string) {
  const info: AppInfo = await invoke('create_database', { path });
  app.dbOpen = true;
  app.dbPath = info.db_path;
  localStorage.setItem('lod-last-db', path);
  await loadAll();
  app.panel = 'welcome';
  app.toolsOpen = false;
  toast('New database created', 'ok');
}
export function getLastDbPath(): string {
  return localStorage.getItem('lod-last-db') || '';
}
export function closeDb() {
  app.dbOpen = false;
  app.dbPath = '';
  app.wordCount = 0;
  app.words = [];
  app.filteredWords = [];
  app.types = [];
  app.authors = [];
  app.events = [];
  app.curWord = null;
  app.curEvent = null;
  app.panel = 'welcome';
  app.toolsOpen = false;
  app.mobileShowList = true;
  app.editing = false;
  app.tab = 'words';
  app.history = [];
  app.historyIdx = -1;
  app.dbStats = null;
  localStorage.removeItem('lod-last-db');
}
export async function loadDbStats() {
  app.dbStats = await invoke('get_db_stats');
}

async function loadAll() {
  try {
    await Promise.all([loadWords(), loadTypes(), loadEvents(), loadAuthors()]);
  } catch (e) {
    console.error('loadAll:', e);
  }
}

export async function loadWords() {
  app.words = await invoke('get_words', {
    q: '',
    typeFilter: '',
    eventId: app.prefs.eventFilter ?? null,
  });
  app.wordCount = app.words.length;
  applyFilter();
}
export function activeEvent(): EventItem | null {
  if (!app.prefs.eventFilter) return null;
  return app.events.find((e) => e.id === app.prefs.eventFilter) ?? null;
}
export function applyFilter() {
  if (!app.words) {
    app.filteredWords = [];
    return;
  }
  const q = app.searchQ.trim().toLowerCase();
  const tf = app.typeFilter;
  let ws = app.words;
  if (q) {
    if (q.includes('*') || q.includes('?')) {
      const pat = new RegExp(
        `^${q
          .replace(/[.+^${}()|[\]\\]/g, '\\$&')
          .replace(/\*/g, '.*')
          .replace(/\?/g, '.')}$`,
        'i'
      );
      ws = ws.filter((w) => pat.test(w.name));
    } else {
      ws = ws.filter((w) => w.name.toLowerCase().startsWith(q));
    }
  }
  if (tf) {
    if (tf.startsWith('__g__')) {
      const g = tf.slice(5);
      ws = ws.filter((w) => app.types.find((t) => t.name === w.type_name)?.group_ === g);
    } else {
      ws = ws.filter((w) => w.type_name === tf);
    }
  }
  app.filteredWords = ws;
}

export async function selectWord(id: number, pushHist = true) {
  if (!id) return;
  try {
    app.curWord = await invoke('get_word', { id });
    app.editing = false;
    app.panel = 'word';
    app.tab = 'words';
    app.mobileShowList = false;
    if (pushHist) _pushHistory({ tab: 'words', id });
    _scrollSidebarTo(id);
  } catch (e) {
    toast('Word not found', 'err');
  }
}
export async function saveWord(id: number | null, data: object) {
  const w: WordDetail = await invoke('save_word', { id, data });
  toast(id ? 'Saved!' : 'Created!', 'ok');
  app.curWord = w;
  app.editing = false;
  app.panel = 'word';
  await loadWords();
}
export async function deleteWord(id: number) {
  await invoke('delete_word', { id });
  toast('Deleted', 'ok');
  app.curWord = null;
  app.panel = 'welcome';
  app.mobileShowList = true;
  await loadWords();
}
export async function saveDef(id: number | null, wordId: number, data: object) {
  app.curWord = await invoke('save_definition', { id, wordId, data });
  toast(id ? 'Updated' : 'Added', 'ok');
}
export async function deleteDef(id: number, wordId: number) {
  app.curWord = await invoke('delete_definition', { id, wordId });
  toast('Deleted', 'ok');
}

export async function loadEvents() {
  app.events = await invoke('get_events');
}
export async function selectEvent(id: number, pushHist = true) {
  app.curEvent = app.events.find((e) => e.id === id) || null;
  app.editing = false;
  app.panel = 'event';
  app.tab = 'events';
  app.mobileShowList = false;
  if (pushHist) _pushHistory({ tab: 'events', id });
}
export async function saveEvent(id: number | null, data: object) {
  const ev: EventItem = await invoke('save_event', { id, data });
  toast(id ? 'Saved!' : 'Created!', 'ok');
  await loadEvents();
  app.curEvent = app.events.find((e) => e.id === ev.id) || null;
  app.editing = false;
  app.panel = 'event';
}
export async function deleteEvent(id: number) {
  await invoke('delete_event', { id });
  toast('Deleted', 'ok');
  app.curEvent = null;
  app.panel = 'welcome';
  app.mobileShowList = true;
  await loadEvents();
}

export async function loadTypes() {
  app.types = await invoke('get_types');
}
export async function saveType(id: number | null, data: object) {
  app.types = await invoke('save_type', { id, data });
  toast(id ? 'Updated!' : 'Created!', 'ok');
}
export async function deleteType(id: number) {
  app.types = await invoke('delete_type', { id });
  toast('Deleted', 'ok');
}

export async function loadAuthors() {
  app.authors = await invoke('get_authors');
}
export async function saveAuthor(id: number | null, data: object) {
  app.authors = await invoke('save_author', { id, data });
  toast(id ? 'Updated!' : 'Added!', 'ok');
}
export async function deleteAuthor(id: number) {
  app.authors = await invoke('delete_author', { id });
  toast('Deleted', 'ok');
}

export async function importFiles(paths: string[]) {
  const result = await invoke('import_lod_files', { paths });
  await loadAll();
  loadDbStats().catch(() => {});
  return result;
}
export async function exportHtmlToFile(path: string, eventName: string | null): Promise<void> {
  await invoke('export_html_to_file', { path, eventName });
}

// ─── E→L Search ──────────────────────────────────────────────────────────────

let _elTimer = 0;
export function setSearchMode(mode: SearchMode) {
  app.searchMode = mode;
  app.elResults = [];
  if (mode === 'le') {
    app.elQuery = '';
  }
}

export async function checkFts() {
  if (!app.dbOpen) return;
  app.elFtsReady = await invoke('fts_is_ready');
}

export async function rebuildFts() {
  if (!app.dbOpen) return;
  toast('Rebuilding FTS index…', 'info');
  try {
    const count: number = await invoke('rebuild_fts');
    app.elFtsReady = true;
    toast(`FTS ready — ${count.toLocaleString()} entries`, 'ok');
  } catch (e) {
    toast(String(e), 'err');
  }
}

export function searchEnglishDebounced(q: string) {
  app.elQuery = q;
  clearTimeout(_elTimer);
  if (!q.trim()) {
    app.elResults = [];
    return;
  }
  _elTimer = setTimeout(() => searchEnglishNow(q), 250) as unknown as number;
}

export async function searchEnglishNow(q = app.elQuery) {
  if (!q.trim() || !app.dbOpen) return;
  app.elSearching = true;
  try {
    app.elResults = await invoke('search_english', {
      params: { query: q, use_like: app.prefs.elUseLike, limit: 300 },
    });
  } catch (e) {
    // FTS may fail on syntax error — re-try with LIKE
    try {
      app.elResults = await invoke('search_english', {
        params: { query: q, use_like: true, limit: 300 },
      });
    } catch {
      app.elResults = [];
    }
  } finally {
    app.elSearching = false;
  }
}

function _pushHistory(entry: { tab: Tab; id: number }) {
  const cur = app.history[app.historyIdx];
  if (cur && cur.tab === entry.tab && cur.id === entry.id) return;
  app.history = app.history.slice(0, app.historyIdx + 1);
  app.history.push(entry);
  if (app.history.length > 100) app.history = app.history.slice(-100);
  app.historyIdx = app.history.length - 1;
}
export async function goBack() {
  // If on types/authors with no history, go to words welcome
  if (app.tab === 'types' || app.tab === 'authors') {
    app.tab = 'words';
    app.panel = app.curWord ? 'word' : 'welcome';
    app.mobileShowList = true;
    return;
  }
  if (app.historyIdx <= 0) return;
  app.historyIdx--;
  const e = app.history[app.historyIdx]!;
  if (e.tab === 'words') await selectWord(e.id, false);
  if (e.tab === 'events') await selectEvent(e.id, false);
}
export async function goForward() {
  if (app.historyIdx >= app.history.length - 1) return;
  app.historyIdx++;
  const e = app.history[app.historyIdx]!;
  if (e.tab === 'words') await selectWord(e.id, false);
  if (e.tab === 'events') await selectEvent(e.id, false);
}
// Can go back if: on types/authors tab (escape to words), or history has previous entry
export function canGoBack() {
  if (app.tab === 'types' || app.tab === 'authors') return true;
  return app.historyIdx > 0;
}
export function canGoForward() {
  return app.historyIdx < app.history.length - 1;
}

function _scrollSidebarTo(id: number) {
  requestAnimationFrame(() => {
    const list = document.getElementById('sb-list');
    if (!list) return;
    const item = document.getElementById(`wi${id}`);
    if (item) {
      item.scrollIntoView({ block: 'nearest' });
    } else {
      const idx = app.filteredWords.findIndex((x) => x.id === id);
      if (idx >= 0) {
        const ROW_H = 28,
          top = idx * ROW_H,
          ch = list.clientHeight;
        if (top < list.scrollTop || top + ROW_H > list.scrollTop + ch)
          list.scrollTop = Math.max(0, top - ch / 2);
      }
    }
  });
}
