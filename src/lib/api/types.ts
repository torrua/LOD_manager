import type {
  WordListItem,
  WordDetail,
  EventItem,
  TypeItem,
  AuthorItem,
  DbStats,
  ELResult,
  ELSearchParams,
} from '../../types';

export interface GetWordsParams {
  q?: string;
  typeFilter?: string;
  eventId?: number | null;
}

export interface DataAdapter {
  getWords(params?: GetWordsParams): Promise<WordListItem[]>;
  getWord(id: number): Promise<WordDetail>;
  getEvents(): Promise<EventItem[]>;
  getTypes(): Promise<TypeItem[]>;
  getAuthors(): Promise<AuthorItem[]>;
  searchEnglish(params: ELSearchParams): Promise<ELResult[]>;
  saveWord(id: number | null, data: object): Promise<WordDetail>;
  deleteWord(id: number): Promise<void>;
  saveDefinition(id: number | null, wordId: number, data: object): Promise<WordDetail>;
  deleteDefinition(id: number, wordId: number): Promise<WordDetail>;
  getDbStats(): Promise<DbStats>;

  // Extended operations supported natively in Tauri and mapped/stubbed in HTTP
  saveEvent?(id: number | null, data: object): Promise<EventItem>;
  deleteEvent?(id: number): Promise<void>;
  saveType?(id: number | null, data: object): Promise<TypeItem[]>;
  deleteType?(id: number): Promise<TypeItem[]>;
  saveAuthor?(id: number | null, data: object): Promise<AuthorItem[]>;
  deleteAuthor?(id: number): Promise<AuthorItem[]>;
  getEventWords?(eventId: number): Promise<[string[], string[]]>;
  ftsIsReady?(): Promise<boolean>;
  rebuildFts?(): Promise<number>;
  compactDb?(): Promise<string>;
  onRevalidate?: ((key: string, data: unknown) => void) | undefined;
}
