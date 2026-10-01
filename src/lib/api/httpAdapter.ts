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
import { getCached, setCached } from './cache';

function getApiBase(): string {
  const envUrl = (import.meta.env.VITE_API_URL as string | undefined) ?? '';
  return envUrl ? envUrl.replace(/\/+$/, '') : '';
}

function getAuthHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (typeof window !== 'undefined' && window.Telegram?.WebApp?.initData) {
    const initData = window.Telegram.WebApp.initData;
    headers['X-Telegram-Init-Data'] = initData;
    headers['Authorization'] = `tma ${initData}`;
  }

  return headers;
}

export class HttpAdapter implements DataAdapter {
  private base = getApiBase();
  onRevalidate?: ((key: string, data: unknown) => void) | undefined;

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.base}${path}`;
    const headers = {
      ...getAuthHeaders(),
      ...(options.headers as Record<string, string> | undefined),
    };

    const res = await fetch(url, {
      ...options,
      headers,
    });

    if (!res.ok) {
      let errText = `HTTP error ${res.status}`;
      try {
        const errJson = (await res.json()) as { error?: string };
        if (errJson.error) errText = errJson.error;
      } catch {
        // Fallback to text
        try {
          const t = await res.text();
          if (t) errText = t;
        } catch {
          // Keep default error text
        }
      }
      throw new Error(errText);
    }

    return res.json() as Promise<T>;
  }

  // ── SWR cached words list ──────────────────────────────────────────────────
  async getWords(params?: GetWordsParams): Promise<WordListItem[]> {
    const cacheKey = `words_ev_${params?.eventId ?? 'all'}`;

    // For full word lists without query filter, apply SWR caching
    if (!params?.q && !params?.typeFilter) {
      const cached = await getCached<WordListItem[]>(cacheKey);

      // Trigger background revalidation
      const fetchPromise = this.fetchWordsFromApi(params)
        .then(async (fresh) => {
          await setCached(cacheKey, fresh);
          if (this.onRevalidate) {
            this.onRevalidate(cacheKey, fresh);
          }
          return fresh;
        })
        .catch((e) => {
          console.warn('HttpAdapter: words background fetch failed:', e);
          return cached ?? [];
        });

      if (cached && cached.length > 0) {
        // Return stale data immediately, revalidation runs in background
        return cached;
      }

      // If no cache, wait for network
      return fetchPromise;
    }

    return this.fetchWordsFromApi(params);
  }

  private async fetchWordsFromApi(params?: GetWordsParams): Promise<WordListItem[]> {
    const query = new URLSearchParams();
    if (params?.q) query.set('q', params.q);
    if (params?.typeFilter) query.set('typeFilter', params.typeFilter);
    if (params?.eventId !== undefined && params?.eventId !== null) {
      query.set('eventId', String(params.eventId));
    }
    const qStr = query.toString();
    const path = `/api/v1/words${qStr ? `?${qStr}` : ''}`;
    const items = await this.request<WordListItem[]>(path);
    return Array.isArray(items) ? items : [];
  }

  async getWord(id: number): Promise<WordDetail> {
    const cacheKey = `word_${id}`;
    try {
      const detail = await this.request<WordDetail>(`/api/v1/words/${id}`);
      await setCached(cacheKey, detail);
      return detail;
    } catch (e) {
      const cached = await getCached<WordDetail>(cacheKey);
      if (cached) return cached;
      throw e;
    }
  }

  async getEvents(): Promise<EventItem[]> {
    const cacheKey = 'events_list';
    try {
      const events = await this.request<EventItem[]>('/api/v1/events');
      const list = Array.isArray(events) ? events : [];
      await setCached(cacheKey, list);
      return list;
    } catch {
      const cached = await getCached<EventItem[]>(cacheKey);
      if (cached && Array.isArray(cached)) return cached;
      return [];
    }
  }

  async getEventWords(eventId: number): Promise<[string[], string[]]> {
    try {
      const data = await this.request<[string[], string[]]>(`/api/v1/events/${eventId}/words`);
      if (Array.isArray(data) && Array.isArray(data[0]) && Array.isArray(data[1])) {
        return data;
      }
      return [[], []];
    } catch (e) {
      console.warn(`Failed to fetch event words for event ${eventId}:`, e);
      return [[], []];
    }
  }

  async getTypes(): Promise<TypeItem[]> {
    const cacheKey = 'types_list';
    try {
      const types = await this.request<TypeItem[]>('/api/v1/types');
      const list = Array.isArray(types) ? types : [];
      await setCached(cacheKey, list);
      return list;
    } catch {
      const cached = await getCached<TypeItem[]>(cacheKey);
      if (cached && Array.isArray(cached)) return cached;
      return [];
    }
  }

  async getAuthors(): Promise<AuthorItem[]> {
    const cacheKey = 'authors_list';
    try {
      const authors = await this.request<AuthorItem[]>('/api/v1/authors');
      const list = Array.isArray(authors) ? authors : [];
      await setCached(cacheKey, list);
      return list;
    } catch {
      const cached = await getCached<AuthorItem[]>(cacheKey);
      if (cached && Array.isArray(cached)) return cached;
      return [];
    }
  }

  async searchEnglish(params: ELSearchParams): Promise<ELResult[]> {
    const query = new URLSearchParams({
      query: params.query,
      use_like: String(params.use_like),
      use_keywords_only: String(params.use_keywords_only),
      limit: String(params.limit || 300),
    });
    return this.request<ELResult[]>(`/api/v1/search/english?${query.toString()}`);
  }

  async saveWord(id: number | null, data: object): Promise<WordDetail> {
    const method = id ? 'PUT' : 'POST';
    const path = id ? `/api/v1/words/${id}` : '/api/v1/words';
    const detail = await this.request<WordDetail>(path, {
      method,
      body: JSON.stringify(data),
    });
    if (id) {
      await setCached(`word_${id}`, detail);
    }
    return detail;
  }

  async deleteWord(id: number): Promise<void> {
    await this.request<void>(`/api/v1/words/${id}`, {
      method: 'DELETE',
    });
  }

  async saveDefinition(id: number | null, wordId: number, data: object): Promise<WordDetail> {
    const method = id ? 'PUT' : 'POST';
    const path = id
      ? `/api/v1/words/${wordId}/definitions/${id}`
      : `/api/v1/words/${wordId}/definitions`;
    const detail = await this.request<WordDetail>(path, {
      method,
      body: JSON.stringify(data),
    });
    await setCached(`word_${wordId}`, detail);
    return detail;
  }

  async deleteDefinition(id: number, wordId: number): Promise<WordDetail> {
    const detail = await this.request<WordDetail>(`/api/v1/words/${wordId}/definitions/${id}`, {
      method: 'DELETE',
    });
    await setCached(`word_${wordId}`, detail);
    return detail;
  }

  async getDbStats(): Promise<DbStats> {
    const cacheKey = 'db_stats';
    try {
      const stats = await this.request<DbStats>('/api/v1/stats');
      await setCached(cacheKey, stats);
      return stats;
    } catch {
      const cached = await getCached<DbStats>(cacheKey);
      if (cached) return cached;
      return {
        db_path:
          this.base || (typeof window !== 'undefined' ? window.location.origin : 'Remote API'),
        word_count: 0,
        definition_count: 0,
        event_count: 0,
        type_count: 0,
        author_count: 0,
        affix_count: 0,
        spelling_count: 0,
        settings: [],
      };
    }
  }

  async ftsIsReady(): Promise<boolean> {
    return true;
  }

  async rebuildFts(): Promise<number> {
    return 0;
  }

  async compactDb(): Promise<string> {
    return '0 B (Server Managed)';
  }
}
