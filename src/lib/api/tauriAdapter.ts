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
import type { DataAdapter, GetWordsParams } from './types';

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke(cmd, args);
}

export class TauriAdapter implements DataAdapter {
  async getWords(params?: GetWordsParams): Promise<WordListItem[]> {
    return tauriInvoke('get_words', {
      q: params?.q ?? '',
      typeFilter: params?.typeFilter ?? '',
      eventId: params?.eventId ?? null,
    });
  }

  async getWord(id: number): Promise<WordDetail> {
    return tauriInvoke('get_word', { id });
  }

  async getEvents(): Promise<EventItem[]> {
    return tauriInvoke('get_events');
  }

  async getTypes(): Promise<TypeItem[]> {
    return tauriInvoke('get_types');
  }

  async getAuthors(): Promise<AuthorItem[]> {
    return tauriInvoke('get_authors');
  }

  async searchEnglish(params: ELSearchParams): Promise<ELResult[]> {
    return tauriInvoke('search_english', { params: params as unknown as Record<string, unknown> });
  }

  async saveWord(id: number | null, data: object): Promise<WordDetail> {
    return tauriInvoke('save_word', { id, data });
  }

  async deleteWord(id: number): Promise<void> {
    return tauriInvoke('delete_word', { id });
  }

  async saveDefinition(id: number | null, wordId: number, data: object): Promise<WordDetail> {
    return tauriInvoke('save_definition', { id, wordId, data });
  }

  async deleteDefinition(id: number, wordId: number): Promise<WordDetail> {
    return tauriInvoke('delete_definition', { id, wordId });
  }

  async getDbStats(): Promise<DbStats> {
    return tauriInvoke('get_db_stats');
  }

  async saveEvent(id: number | null, data: object): Promise<EventItem> {
    return tauriInvoke('save_event', { id, data });
  }

  async deleteEvent(id: number): Promise<void> {
    return tauriInvoke('delete_event', { id });
  }

  async saveType(id: number | null, data: object): Promise<TypeItem[]> {
    return tauriInvoke('save_type', { id, data });
  }

  async deleteType(id: number): Promise<TypeItem[]> {
    return tauriInvoke('delete_type', { id });
  }

  async saveAuthor(id: number | null, data: object): Promise<AuthorItem[]> {
    return tauriInvoke('save_author', { id, data });
  }

  async deleteAuthor(id: number): Promise<AuthorItem[]> {
    return tauriInvoke('delete_author', { id });
  }

  async getEventWords(eventId: number): Promise<[string[], string[]]> {
    return tauriInvoke('get_event_words', { eventId });
  }

  async ftsIsReady(): Promise<boolean> {
    return tauriInvoke('fts_is_ready');
  }

  async rebuildFts(): Promise<number> {
    return tauriInvoke('rebuild_fts');
  }

  async compactDb(): Promise<string> {
    return tauriInvoke('compact_db');
  }
}
