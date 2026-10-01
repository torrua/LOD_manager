import { adapter, isTauri } from './api';
import {
  getPlatform,
  readBinaryFile,
  writeBinaryFile,
  getAppDataDirPath,
  checkAppUpdate,
  relaunchApp,
} from './tauriBridge';
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
  ImportResult,
} from '../types';
import { findAffixWord, findWordByName } from './text';

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
  isAdmin: isTauri as boolean,
  readonly: isTauri ? localStorage.getItem('lod-ro') === '1' : true,
  tab: 'words' as Tab,
  words: [] as WordListItem[],
  filteredWords: [] as WordListItem[],
  searchQ: '',
  typeFilter: '',
  curWord: null as WordDetail | null,
  loadingWordId: null as number | null,
  curEvent: null as EventItem | null,
  types: [] as TypeItem[],
  authors: [] as AuthorItem[],
  events: [] as EventItem[],
  panel: 'welcome' as string,
  editing: false,
  history: [] as Array<{ tab: Tab; id: number }>,
  historyIdx: -1,
  toolsOpen: false,
  toolsTab: 'settings' as 'import' | 'export' | 'database' | 'settings',
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
  currentPlatform: 'unknown' as string,
  suggestImport: false, // set true when DB opened/created empty
  updateAvailable: false,
  updateVersion: '',
  updateDownloading: false,
  updateProgress: 0,
  debugLog: [] as string[],
  debugVisible: false,
  impResult: null as ImportResult | null,
  prefs: {
    showTypeTag: (loadPrefs().showTypeTag ?? true) as boolean,
    showDefCount: (loadPrefs().showDefCount ?? true) as boolean,
    visibleMeta: (loadPrefs().visibleMeta ?? [...DEFAULT_META]) as string[],
    ...loadPrefs(),
    elShowSnippet: (loadPrefs().elShowSnippet ?? true) as boolean,
    elShowGrammar: (loadPrefs().elShowGrammar ?? true) as boolean,
    elShowType: (loadPrefs().elShowType ?? true) as boolean,
    elShowCount: (loadPrefs().elShowCount ?? true) as boolean,
    elUseLike: (loadPrefs().elUseLike ?? false) as boolean,
    elUseKeywords: (loadPrefs().elUseKeywords ?? false) as boolean,
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
    elUseKeywords: boolean;
    elShowDetails: boolean;
    eventFilter: number | null;
    showTooltips: boolean;
  },
});

export function setPref<K extends keyof typeof app.prefs>(k: K, v: (typeof app.prefs)[K]) {
  app.prefs[k] = v;
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
  if (!app.isAdmin) {
    toast('Editing is restricted to administrators', 'info');
    return;
  }
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

/// Ask Rust for the canonical default DB path (app_data_dir/lod.db).
/// This works reliably on Android where JS path construction can mismatch.
export async function getDefaultDbPath(): Promise<string> {
  if (!isTauri) return '';
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('get_default_db_path');
}

export async function initWebMode() {
  app.dbOpen = true;
  const envUrl = (import.meta.env.VITE_API_URL as string | undefined) ?? '';
  app.dbPath = envUrl || (typeof window !== 'undefined' ? window.location.origin : 'Remote API');
  try {
    if (adapter.checkAdminStatus) {
      app.isAdmin = await adapter.checkAdminStatus();
    } else {
      app.isAdmin = false;
    }
    if (!app.isAdmin) {
      app.readonly = true;
    }
    await loadAll();
    await autoSelectLatestEvent();
    loadDbStats().catch(() => {});
    checkFts().catch(() => {});
  } catch (e) {
    console.error('initWebMode failed:', e);
    toast(`Failed to load: ${String(e)}`, 'err');
  }
}

export async function openDb(path: string) {
  if (!isTauri) {
    await initWebMode();
    return;
  }
  // Android file picker returns content:// URIs which SQLite cannot open
  // directly. Copy the file into app_data_dir as lod.db (the canonical path)
  // so that it survives app restarts and updates.
  let actualPath = path;
  if (path.startsWith('content://')) {
    try {
      const bytes = await readBinaryFile(path);
      const destName = 'lod.db';
      await writeBinaryFile(destName, bytes);
      const dir = await getAppDataDirPath();
      actualPath = dir.endsWith('/') ? `${dir}${destName}` : `${dir}/${destName}`;
    } catch (e) {
      throw new Error(
        `Cannot read Android file: ${String(e)}. Try using "Import" from the Tools menu instead.`
      );
    }
  }
  const { invoke } = await import('@tauri-apps/api/core');
  const info: AppInfo = await invoke('open_database', { path: actualPath });
  app.dbOpen = true;
  app.dbPath = info.db_path;
  // Only persist non-Android paths; Android always derives path from app_data_dir
  if (!actualPath.startsWith('content://')) {
    localStorage.setItem('lod-last-db', actualPath);
  }
  await loadAll();
  await autoSelectLatestEvent();
  app.panel = 'welcome';
  app.toolsOpen = false;
  toast(`Opened — ${app.words.length.toLocaleString()} words`, 'ok');
  if (app.words.length === 0) app.suggestImport = true;
  checkFts().catch(() => {});
  loadDbStats().catch(() => {});
}

export async function createDb(path: string) {
  if (!isTauri) return;
  const { invoke } = await import('@tauri-apps/api/core');
  const info: AppInfo = await invoke('create_database', { path });
  app.dbOpen = true;
  app.dbPath = info.db_path;
  localStorage.setItem('lod-last-db', path);
  await loadAll();
  await autoSelectLatestEvent();
  app.panel = 'welcome';
  app.toolsOpen = false;
  toast('New database created', 'ok');
  app.suggestImport = true;
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
  app.loadingWordId = null;
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
  app.dbStats = await adapter.getDbStats();
}

async function loadAll() {
  try {
    await Promise.allSettled([loadWords(), loadTypes(), loadEvents(), loadAuthors()]);
  } catch (e) {
    console.error('loadAll:', e);
  }
}

export async function loadWords() {
  try {
    const words = await adapter.getWords({
      q: '',
      typeFilter: '',
      eventId: app.prefs.eventFilter ?? null,
    });
    app.words = Array.isArray(words) ? words : [];
    app.wordCount = app.words.length;
    applyFilter();
  } catch (error) {
    console.error('loadWords: error:', error);
    app.words = [];
    app.wordCount = 0;
    applyFilter();
  }
}

if ('onRevalidate' in adapter) {
  adapter.onRevalidate = (key: string, data: unknown) => {
    if (key.startsWith('words_')) {
      const freshWords = data as WordListItem[];
      if (
        freshWords &&
        (freshWords.length !== app.words.length ||
          freshWords[0]?.id !== app.words[0]?.id ||
          freshWords[freshWords.length - 1]?.id !== app.words[app.words.length - 1]?.id)
      ) {
        app.words = freshWords;
        app.wordCount = app.words.length;
        applyFilter();
      }
    }
  };
}

// Reactive derived — tracks app.events and app.prefs.eventFilter automatically.
export const getActiveEvent = () =>
  app.prefs.eventFilter ? (app.events.find((e) => e.id === app.prefs.eventFilter) ?? null) : null;

// Cache for type-group lookups so applyFilter doesn't scan app.types on every word
const _typeGroupCache = new Map<string, string | undefined>();
let _typeGroupCacheStamp = 0;

export function applyFilter() {
  if (!app.words) {
    app.filteredWords = [];
    return;
  }
  const q = app.searchQ.trim().toLowerCase();
  const tf = app.typeFilter;
  // Fast path: no filters → assign reference directly (zero allocation)
  if (!q && !tf) {
    app.filteredWords = app.words;
    return;
  }
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
      if (_typeGroupCacheStamp !== app.types.length) {
        _typeGroupCache.clear();
        for (const t of app.types) _typeGroupCache.set(t.name, t.group_ || undefined);
        _typeGroupCacheStamp = app.types.length;
      }
      ws = ws.filter((w) => w.type_name !== null && _typeGroupCache.get(w.type_name) === g);
    } else {
      ws = ws.filter((w) => w.type_name === tf);
    }
  }
  app.filteredWords = ws;
}

export async function selectWord(id: number, pushHist = true) {
  if (!id) return;
  if (app.loadingWordId === id) return;
  app.loadingWordId = id;
  app.tab = 'words';
  try {
    const word: WordDetail = await adapter.getWord(id);
    if (app.loadingWordId !== id) return;
    app.curWord = word;
    app.editing = false;
    app.panel = 'word';
    if (pushHist) pushHistory({ tab: 'words', id });
  } catch (error) {
    console.error('selectWord: error:', error);
    toast('Word not found', 'err');
    if (app.loadingWordId === id) app.mobileShowList = true;
  } finally {
    if (app.loadingWordId === id) app.loadingWordId = null;
  }
}

function getAffixTypeSet(): Set<string> {
  const affixTypes = new Set<string>(['Afx', 'Affix']);
  for (const t of app.types) {
    if (t.type_x === 'Affix' || t.group_ === 'Affix') {
      affixTypes.add(t.name);
    }
  }
  return affixTypes;
}

export async function selectWordByName(name: string) {
  const affixTypes = getAffixTypeSet();
  const w = findWordByName(app.words, name, affixTypes);
  if (w) {
    await selectWord(w.id);
    return;
  }
  const clean = name.trim().replace(/^-+|-+$/g, '');
  if (!clean) return;
  try {
    const q = clean.includes('*') || clean.includes('?') ? clean : `*${clean}*`;
    const matches = await adapter.getWords({
      q,
      typeFilter: '',
      eventId: null,
    });
    const exact = findWordByName(matches, name, affixTypes);
    if (exact) {
      await selectWord(exact.id);
    }
  } catch (error) {
    console.error('selectWordByName: error:', error);
  }
}

export async function selectAffixByName(affix: string) {
  const affixTypes = getAffixTypeSet();

  const found = findAffixWord(app.words, affix, affixTypes);
  if (found) {
    await selectWord(found.id);
    return;
  }

  const clean = affix.trim().replace(/^-+|-+$/g, '');
  if (!clean) return;

  try {
    const q = clean.includes('*') || clean.includes('?') ? clean : `*${clean}*`;
    const matches = await adapter.getWords({
      q,
      typeFilter: '',
      eventId: null,
    });
    const dbMatch = findAffixWord(matches, affix, affixTypes);
    if (dbMatch) {
      await selectWord(dbMatch.id);
      return;
    }
  } catch (error) {
    console.error('selectAffixByName: error:', error);
  }

  // Fallback to search filter
  app.tab = 'words';
  app.searchQ = clean;
  applyFilter();
  const filteredMatch = findAffixWord(app.filteredWords, affix, affixTypes) ?? app.filteredWords[0];
  if (filteredMatch) {
    await selectWord(filteredMatch.id);
  }
}

export async function saveWord(id: number | null, data: object) {
  if (app.readonly) return;
  const w: WordDetail = await adapter.saveWord(id, data);
  toast(id ? 'Saved!' : 'Created!', 'ok');
  app.curWord = w;
  app.editing = false;
  app.panel = 'word';
  await loadWords();
}

export async function deleteWord(id: number) {
  if (app.readonly) return;
  await adapter.deleteWord(id);
  toast('Deleted', 'ok');
  app.curWord = null;
  app.panel = 'welcome';
  app.mobileShowList = true;
  await loadWords();
}

export async function saveDef(id: number | null, wordId: number, data: object) {
  if (app.readonly) return;
  app.curWord = await adapter.saveDefinition(id, wordId, data);
  toast(id ? 'Updated' : 'Added', 'ok');
}

export async function deleteDef(id: number, wordId: number) {
  if (app.readonly) return;
  app.curWord = await adapter.deleteDefinition(id, wordId);
  toast('Deleted', 'ok');
}

export async function loadEvents() {
  try {
    const evs = await adapter.getEvents();
    app.events = Array.isArray(evs) ? evs : [];
  } catch (e) {
    console.error('loadEvents error:', e);
    app.events = [];
  }
}

// Automatically select the latest event if no filter is set
export async function autoSelectLatestEvent() {
  if (!app.events || app.events.length === 0) return;

  // Only auto-select if user hasn't already set a filter
  if (app.prefs.eventFilter !== null) return;

  let latestEvent: EventItem | undefined;

  // First try to find the latest event by valid non-empty date
  const eventsWithDate = app.events.filter(
    (e) => e.date !== null && e.date.trim() !== '' && !Number.isNaN(new Date(e.date).getTime())
  );
  if (eventsWithDate.length > 0) {
    latestEvent = [...eventsWithDate].sort(
      (a, b) => new Date(b.date ?? 0).getTime() - new Date(a.date ?? 0).getTime()
    )[0];
  } else {
    // Fallback: use the event with the highest ID without mutating app.events in-place
    latestEvent = [...app.events].sort((a, b) => b.id - a.id)[0];
  }

  if (latestEvent) {
    setPref('eventFilter', latestEvent.id);
    await loadWords(); // Refresh word list with new filter
  }
}

export async function selectEvent(id: number, pushHist = true) {
  app.curEvent = app.events.find((e) => e.id === id) || null;
  app.editing = false;
  app.panel = 'event';
  app.tab = 'events';
  app.mobileShowList = false;
  if (pushHist) pushHistory({ tab: 'events', id });
}

export async function saveEvent(id: number | null, data: object) {
  if (app.readonly) return;
  if (adapter.saveEvent) {
    const ev = await adapter.saveEvent(id, data);
    toast(id ? 'Saved!' : 'Created!', 'ok');
    await loadEvents();
    app.curEvent = app.events.find((e) => e.id === ev.id) || null;
    app.editing = false;
    app.panel = 'event';
  } else if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    const ev: EventItem = await invoke('save_event', { id, data });
    toast(id ? 'Saved!' : 'Created!', 'ok');
    await loadEvents();
    app.curEvent = app.events.find((e) => e.id === ev.id) || null;
    app.editing = false;
    app.panel = 'event';
  }
}

export async function deleteEvent(id: number) {
  if (app.readonly) return;
  if (adapter.deleteEvent) {
    await adapter.deleteEvent(id);
  } else if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('delete_event', { id });
  }
  toast('Deleted', 'ok');
  app.curEvent = null;
  app.panel = 'welcome';
  app.mobileShowList = true;
  await loadEvents();
}

export async function loadTypes() {
  try {
    const types = await adapter.getTypes();
    app.types = Array.isArray(types) ? types : [];
  } catch (e) {
    console.error('loadTypes error:', e);
    app.types = [];
  }
}

export async function saveType(id: number | null, data: object) {
  if (app.readonly) return;
  if (adapter.saveType) {
    app.types = await adapter.saveType(id, data);
  } else if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    app.types = await invoke('save_type', { id, data });
  }
  toast(id ? 'Updated!' : 'Created!', 'ok');
}

export async function deleteType(id: number) {
  if (app.readonly) return;
  if (adapter.deleteType) {
    app.types = await adapter.deleteType(id);
  } else if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    app.types = await invoke('delete_type', { id });
  }
  toast('Deleted', 'ok');
}

export async function loadAuthors() {
  try {
    const authors = await adapter.getAuthors();
    app.authors = Array.isArray(authors) ? authors : [];
  } catch (e) {
    console.error('loadAuthors error:', e);
    app.authors = [];
  }
}

export async function saveAuthor(id: number | null, data: object) {
  if (app.readonly) return;
  if (adapter.saveAuthor) {
    app.authors = await adapter.saveAuthor(id, data);
  } else if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    app.authors = await invoke('save_author', { id, data });
  }
  toast(id ? 'Updated!' : 'Added!', 'ok');
}

export async function deleteAuthor(id: number) {
  if (app.readonly) return;
  if (adapter.deleteAuthor) {
    app.authors = await adapter.deleteAuthor(id);
  } else if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    app.authors = await invoke('delete_author', { id });
  }
  toast('Deleted', 'ok');
}

export async function importFiles(paths: string[], fileNames?: string[]) {
  if (!isTauri) {
    throw new Error('Import is only supported on Desktop / Android');
  }
  const { invoke } = await import('@tauri-apps/api/core');

  // Check for different URI types
  const hasContentUris = paths.some(
    (p) => p.startsWith('content://') || p.startsWith('msf:') || p.includes('%3A')
  );
  const hasGitHubUris = paths.some((p) => p.startsWith('github://'));

  let result;
  if (hasGitHubUris) {
    const files: [string, string][] = [];
    for (let i = 0; i < paths.length; i++) {
      const p = paths[i];
      if (!p || !p.startsWith('github://')) continue;

      const colonIndex = p.indexOf(':', 9);
      if (colonIndex === -1) continue;

      const name = p.substring(9, colonIndex);
      const content = p.substring(colonIndex + 1);

      files.push([name, content]);
    }
    result = await invoke('import_lod_contents', { files });
  } else if (hasContentUris) {
    const files: [string, string][] = [];
    for (let i = 0; i < paths.length; i++) {
      const p = paths[i];
      if (!p) continue;
      let name = fileNames?.[i];
      if (!name) {
        if (p.startsWith('content://')) {
          const decoded = decodeURIComponent(p);
          const uriParts = decoded.split('/');
          const lastPart = uriParts[uriParts.length - 1];
          const cleanName = lastPart?.split('?')[0]?.split('#')[0] || '';
          if (cleanName && (cleanName.includes('.') || cleanName.length > 10)) {
            name = cleanName;
          } else {
            const docId = uriParts.find((part) => part.includes('document'))?.split('document/')[1];
            name = docId ? `file_${docId}.txt` : `android_file_${Date.now()}_${i}.txt`;
          }
        } else {
          name = p.split(/[/\\]/).pop() || `file_${i}.txt`;
        }
      }
      if (!name.endsWith('.txt')) name = `${name}.txt`;
      try {
        const bytes = await readBinaryFile(p);
        const text = new TextDecoder('utf-8').decode(bytes);
        files.push([name, text]);
      } catch (e) {
        console.warn('Could not read import file:', p, e);
      }
    }
    result = await invoke('import_lod_contents', { files });
  } else {
    result = await invoke('import_lod_files', { paths });
  }
  await loadAll();
  await autoSelectLatestEvent();
  loadDbStats().catch(() => {});
  return result;
}

export async function convertTextFiles(textDir: string): Promise<ImportResult> {
  if (!isTauri) {
    throw new Error('Converter is only supported on Desktop / Android');
  }
  const { invoke } = await import('@tauri-apps/api/core');
  const result = (await invoke('convert_text_files', { textDir })) as ImportResult;
  await loadAll();
  await autoSelectLatestEvent();
  loadDbStats().catch(() => {});
  return result;
}

export async function exportHtmlToFile(path: string, eventName: string | null): Promise<void> {
  if (!isTauri) {
    throw new Error('Export is only supported on Desktop / Android');
  }
  const { invoke } = await import('@tauri-apps/api/core');
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
  app.elFtsReady = adapter.ftsIsReady ? await adapter.ftsIsReady() : true;
}

export async function rebuildFts() {
  if (!app.dbOpen) return;
  toast('Rebuilding FTS index…', 'info');
  try {
    const count: number = adapter.rebuildFts ? await adapter.rebuildFts() : 0;
    app.elFtsReady = true;
    toast(`FTS ready — ${count.toLocaleString()} entries`, 'ok');
  } catch (e) {
    toast(String(e), 'err');
  }
}

export async function compactDb() {
  if (!app.dbOpen) return;
  toast('Compacting database…', 'info');
  try {
    const size: string = adapter.compactDb ? await adapter.compactDb() : '0 B';
    toast(`Database compacted — ${size}`, 'ok');
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
    app.elResults = await adapter.searchEnglish({
      query: q,
      use_like: app.prefs.elUseLike,
      use_keywords_only: app.prefs.elUseKeywords,
      limit: 300,
    });
  } catch {
    // FTS may fail on syntax error — re-try with LIKE
    try {
      app.elResults = await adapter.searchEnglish({
        query: q,
        use_like: true,
        use_keywords_only: app.prefs.elUseKeywords,
        limit: 300,
      });
    } catch {
      app.elResults = [];
    }
  } finally {
    app.elSearching = false;
  }
}

export function pushHistory(entry: { tab: Tab; id: number }) {
  const cur = app.history[app.historyIdx];
  if (cur && cur.tab === entry.tab && cur.id === entry.id) return;
  app.history = app.history.slice(0, app.historyIdx + 1);
  app.history.push(entry);
  if (app.history.length > 100) app.history = app.history.slice(-100);
  app.historyIdx = app.history.length - 1;
}
export async function goBack() {
  if (app.historyIdx <= 0) return;
  app.historyIdx--;
  const e = app.history[app.historyIdx];
  if (e?.tab === 'words') await selectWord(e.id, false);
  if (e?.tab === 'events') await selectEvent(e.id, false);
}
export async function goForward() {
  if (app.historyIdx >= app.history.length - 1) return;
  app.historyIdx++;
  const e = app.history[app.historyIdx];
  if (e?.tab === 'words') await selectWord(e.id, false);
  if (e?.tab === 'events') await selectEvent(e.id, false);
}
export const canGoBack = () => app.historyIdx > 0;
export const canGoForward = () => app.historyIdx < app.history.length - 1;

export async function getEventWords(eventId: number): Promise<[string[], string[]]> {
  if (adapter.getEventWords) {
    return adapter.getEventWords(eventId);
  }
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_event_words', { eventId });
  }
  return [[], []];
}

export async function initPlatform() {
  try {
    app.currentPlatform = await getPlatform();
  } catch {
    app.currentPlatform = 'unknown';
  }
}

export async function checkForUpdate() {
  if (!isTauri || app.currentPlatform === 'android' || app.currentPlatform === 'ios') return;

  const log = (msg: string) => {
    const timestamp = new Date().toLocaleTimeString();
    app.debugLog = [...app.debugLog.slice(-29), `[${timestamp}] ${msg}`];
  };

  log('Starting update check...');
  log(`Platform: ${app.currentPlatform}`);

  // Call Rust debug endpoint for more verbose logging
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const rustResult: string = await invoke('debug_update_check');
    log(`Rust check: ${rustResult}`);
  } catch (e) {
    log(`Rust check error: ${String(e)}`);
  }

  try {
    const update = await checkAppUpdate();
    if (update) {
      app.updateAvailable = true;
      app.updateVersion = update.version;
      log(`Update available: ${update.version}`);
      toast(`Update ${update.version} available`, 'info');
    } else {
      log('Already up to date');
      toast('Already up to date', 'info');
    }
  } catch (e) {
    console.error('Update check failed:', e);
    log(`ERROR: ${String(e)}`);
    const errMsg = e instanceof Error ? e.message : String(e);
    toast(`Update failed: ${errMsg}`, 'err');
  }
}

export async function installUpdate() {
  if (!isTauri) return;
  try {
    app.updateDownloading = true;
    app.updateProgress = 0;
    const update = await checkAppUpdate();
    if (!update) {
      app.updateDownloading = false;
      return;
    }
    let downloaded = 0;
    let contentLength = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          contentLength = event.data.contentLength ?? 0;
          break;
        case 'Progress':
          downloaded += event.data.chunkLength ?? 0;
          if (contentLength) {
            app.updateProgress = Math.round((downloaded / contentLength) * 100);
          }
          break;
        case 'Finished':
          app.updateProgress = 100;
          break;
      }
    });
    await relaunchApp();
  } catch (e) {
    console.error('Update install failed:', e);
    toast('Update failed', 'err');
    app.updateDownloading = false;
  }
}
