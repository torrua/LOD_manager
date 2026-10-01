import type { DataAdapter } from './types';
import { TauriAdapter } from './tauriAdapter';
import { HttpAdapter } from './httpAdapter';

export * from './types';
export * from './cache';

export const isTauri: boolean =
  typeof window !== 'undefined' &&
  ('__TAURI_INTERNALS__' in window || ('isTauri' in window && Boolean(window.isTauri)));

export const isTelegram: boolean =
  !isTauri &&
  typeof window !== 'undefined' &&
  Boolean(window.Telegram?.WebApp?.initData && window.Telegram.WebApp.initData.length > 0);

export const isWeb: boolean = !isTauri && !isTelegram;

export const adapter: DataAdapter = isTauri ? new TauriAdapter() : new HttpAdapter();
