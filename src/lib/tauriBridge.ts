import { isTauri, isTelegram } from './api';

export async function openFilePicker(options?: {
  title?: string;
  multiple?: boolean;
  directory?: boolean;
  filters?: Array<{ name: string; extensions: string[] }>;
}): Promise<string | string[] | null> {
  if (!isTauri) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  return open(options);
}

export async function saveFileDialog(options?: {
  title?: string;
  defaultPath?: string;
  filters?: Array<{ name: string; extensions: string[] }>;
}): Promise<string | null> {
  if (!isTauri) return null;
  const { save } = await import('@tauri-apps/plugin-dialog');
  return save(options);
}

export async function getAppVersion(): Promise<string> {
  if (!isTauri) return '1.7.1 (Web)';
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    return await getVersion();
  } catch {
    return '—';
  }
}

export async function getPlatform(): Promise<string> {
  if (!isTauri) {
    return isTelegram ? 'telegram' : 'web';
  }
  try {
    const { platform } = await import('@tauri-apps/plugin-os');
    return platform();
  } catch {
    return 'unknown';
  }
}

export async function readBinaryFile(path: string): Promise<Uint8Array> {
  if (!isTauri) {
    throw new Error('Local file operations are not supported in web/TMA mode');
  }
  const { readFile } = await import('@tauri-apps/plugin-fs');
  return readFile(path);
}

export async function writeBinaryFile(destName: string, bytes: Uint8Array): Promise<void> {
  if (!isTauri) {
    throw new Error('Local file operations are not supported in web/TMA mode');
  }
  const { writeFile } = await import('@tauri-apps/plugin-fs');
  const { BaseDirectory } = await import('@tauri-apps/api/path');
  await writeFile(destName, bytes, { baseDir: BaseDirectory.AppData });
}

export async function getAppDataDirPath(): Promise<string> {
  if (!isTauri) return '';
  const { appDataDir } = await import('@tauri-apps/api/path');
  return appDataDir();
}

export interface UpdateDownloadEvent {
  event: 'Started' | 'Progress' | 'Finished';
  data: {
    chunkLength?: number | undefined;
    contentLength?: number | undefined;
  };
}

export async function checkAppUpdate(): Promise<{
  version: string;
  downloadAndInstall: (onEvent?: (event: UpdateDownloadEvent) => void) => Promise<void>;
} | null> {
  if (!isTauri) return null;
  const { check } = await import('@tauri-apps/plugin-updater');
  const update = await check();
  if (!update) return null;

  return {
    version: update.version,
    downloadAndInstall: (onEvent) =>
      update.downloadAndInstall((e) => {
        if (!onEvent) return;
        if (e.event === 'Started') {
          onEvent({
            event: 'Started',
            data: { contentLength: e.data.contentLength },
          });
        } else if (e.event === 'Progress') {
          onEvent({
            event: 'Progress',
            data: { chunkLength: e.data.chunkLength },
          });
        } else if (e.event === 'Finished') {
          onEvent({
            event: 'Finished',
            data: {},
          });
        }
      }),
  };
}

export async function relaunchApp(): Promise<void> {
  if (!isTauri) return;
  const { relaunch } = await import('@tauri-apps/plugin-process');
  await relaunch();
}
