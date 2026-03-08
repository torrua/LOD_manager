export interface WordListItem {
  id: number; name: string; type_name: string | null; def_count: number;
}
export interface Definition {
  id: number; position: number; grammar: string | null;
  usage: string | null; body: string; tags: string | null;
}
export interface WordDetail {
  id: number; name: string; type_name: string | null; type_id: number | null;
  source: string | null; year: string | null; rank: string | null; match_: string | null;
  origin: string | null; origin_x: string | null; notes: string | null;
  event_start_name: string | null; event_end_name: string | null;
  affixes: string[]; spellings: string[]; definitions: Definition[]; used_in: string[];
}
export interface EventItem {
  id: number; name: string; date: string | null;
  annotation: string | null; suffix: string | null; notes: string | null;
}
export interface TypeItem {
  id: number; name: string; type_x: string | null; group_: string | null; word_count: number;
}
export interface AuthorItem {
  id: number; initials: string; full_name: string | null; notes: string | null; word_count: number;
}
export interface ImportResult {
  words: number; definitions: number; events: number;
  types: number; authors: number; settings: number; errors: number; messages: string[];
}
export interface SettingItem { key: string; value: string; }
export interface DbStats {
  db_path: string; word_count: number; definition_count: number;
  event_count: number; type_count: number; author_count: number;
  affix_count: number; spelling_count: number; settings: SettingItem[];
}
export interface AppInfo { db_path: string; word_count: number; definition_count: number; }
export type Tab = 'words' | 'events' | 'types' | 'authors';

export interface ELResult {
  word_id:     number;
  word_name:   string;
  type_name:   string | null;
  grammar:     string | null;
  snippet:     string;
  match_count: number;
}

export interface ELSearchParams {
  query:    string;
  use_like: boolean;
  limit:    number;
}

export type SearchMode = 'le' | 'el';
