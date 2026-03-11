<script lang="ts">
  import Icon from './Icon.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { importFiles, toast } from '../store.svelte';
  import type { ImportResult } from '../../types';

  let paths = $state<string[]>([]);
  let names = $state<string[]>([]); // display filenames (for Android content:// URIs)
  let editingIdx = $state<number | null>(null); // index of file being edited
  let result = $state<ImportResult | null>(null);
  let running = $state(false);
  let downloading = $state(false);
  let githubUrl = $state('https://github.com/torrua/LOD/tree/master/tables');

  async function pickFiles() {
    const selected = await open({
      multiple: true,
      title: 'Select LOD text files',
      filters: [{ name: 'Text', extensions: ['txt'] }],
    });
    if (selected) {
      paths = Array.isArray(selected) ? selected : [selected];
      // Extract display names — suggest common LOD files if detection fails
      names = paths.map((p, idx) => {
        const lodFileNames = [
          'Word.txt',
          'WordDef.txt',
          'WordSpell.txt',
          'LexEvent.txt',
          'type.txt',
          'author.txt',
          'settings.txt',
        ];

        const decoded = decodeURIComponent(p);

        // Try standard file path extraction first
        if (!p.startsWith('content://') && !p.startsWith('msf:') && !p.includes('%3A')) {
          const basename = p.split(/[/\\]/).pop() || '';
          if (basename && basename.includes('.')) {
            return basename;
          }
        }

        // For content:// URIs, try to extract real filename
        if (p.startsWith('content://')) {
          const uriParts = decoded.split('/');
          const lastPart = uriParts[uriParts.length - 1];
          const cleanName = lastPart?.split('?')[0]?.split('#')[0] || '';
          if (cleanName && (cleanName.includes('.') || cleanName.length > 10)) {
            return cleanName;
          }
        }

        // Fallback: suggest common LOD file names in order
        // User can click to edit if wrong
        const defaultName = idx < lodFileNames.length ? lodFileNames[idx] : `file_${idx + 1}.txt`;
        return defaultName || 'file.txt';
      });
      result = null;
    }
  }

  function remove(p: string) {
    const idx = paths.indexOf(p);
    paths = paths.filter((x) => x !== p);
    if (idx >= 0) names = names.filter((_, i) => i !== idx);
  }
  function basename(p: string) {
    return p.split(/[\\/]/).pop() || p;
  }

  function updateName(idx: number, newName: string) {
    if (!newName) return;
    names[idx] = newName;
    if (!newName.endsWith('.txt')) {
      names[idx] = `${newName}.txt`;
    }
  }

  async function downloadFromGitHub() {
    if (downloading) return;
    downloading = true;

    try {
      // Parse GitHub URL
      const urlMatch = githubUrl.match(/github\.com\/([^/]+)\/([^/]+)\/tree\/([^/]+)\/(.+)/);
      if (!urlMatch) {
        throw new Error(
          'Invalid GitHub URL format. Expected: https://github.com/owner/repo/tree/branch/path'
        );
      }

      const [, owner, repo, branch, path] = urlMatch;

      // Get directory contents from GitHub API
      const apiUrl = `https://api.github.com/repos/${owner}/${repo}/contents/${path}?ref=${branch}`;
      const response = await fetch(apiUrl);

      if (!response.ok) {
        throw new Error(`GitHub API error: ${response.status} ${response.statusText}`);
      }

      const files: { type: string; name: string; download_url: string }[] = await response.json();

      if (!Array.isArray(files)) {
        throw new Error('Expected directory contents, got file');
      }

      // Filter only .txt files
      const txtFiles = files.filter(
        (file: { type: string; name: string; download_url: string }) =>
          file.type === 'file' && file.name.endsWith('.txt')
      );

      if (txtFiles.length === 0) {
        throw new Error('No .txt files found in the directory');
      }

      // Download each file
      const downloadedFiles: string[] = [];
      const downloadedNames: string[] = [];

      for (const file of txtFiles) {
        try {
          const fileResponse = await fetch(file.download_url);
          if (!fileResponse.ok) {
            console.warn(`Failed to download ${file.name}: ${fileResponse.status}`);
            continue;
          }

          const content = await fileResponse.text();

          // Create a temporary file-like entry
          // We'll store the content in a special format that importFiles can handle
          const tempPath = `github://${file.name}:${content}`;
          downloadedFiles.push(tempPath);
          downloadedNames.push(file.name);
        } catch (e) {
          console.warn(`Error downloading ${file.name}:`, e);
        }
      }

      if (downloadedFiles.length === 0) {
        throw new Error('Failed to download any files');
      }

      // Add to existing files or replace
      paths = [...paths, ...downloadedFiles];
      names = [...names, ...downloadedNames];

      toast(
        `Downloaded ${downloadedFiles.length} file${downloadedFiles.length > 1 ? 's' : ''} from GitHub`,
        'ok'
      );
    } catch (e) {
      toast(`Download failed: ${String(e)}`, 'err');
    } finally {
      downloading = false;
    }
  }

  async function run() {
    if (!paths.length || running) return;
    running = true;
    result = null;
    try {
      result = (await importFiles(paths, names)) as ImportResult;
      toast(`Import complete: ${result.words} words`, result.errors ? 'err' : 'ok');
    } catch (e) {
      toast(String(e), 'err');
    } finally {
      running = false;
    }
  }
</script>

<div class="ip">
  <div class="sec" style="margin-bottom:.65rem">
    <span class="sec-title">Import LOD Text Files</span>
  </div>

  <p class="ip-help">
    Select the LOD export files (<code>Word.txt</code>, <code>WordSpell.txt</code>,
    <code>WordDef.txt</code>, <code>LexEvent.txt</code>, <code>type.txt</code>,
    <code>author.txt</code>). Files are matched by name — you can select any subset.
  </p>

  <!-- GitHub Download Section -->
  <div class="github-section">
    <div class="github-input-group">
      <input
        type="url"
        class="github-url-input"
        placeholder="https://github.com/owner/repo/tree/branch/path"
        value={githubUrl}
        onchange={(e) => {
          const target = e.target as HTMLInputElement;
          if (target) githubUrl = target.value;
        }}
        disabled={downloading}
      />
      <button class="btn btn-g" onclick={downloadFromGitHub} disabled={downloading}>
        {downloading ? 'Downloading…' : 'Download from GitHub'}
      </button>
    </div>
    <p class="github-help">Download LOD files directly from a GitHub repository directory</p>
  </div>

  <div class="file-zone" class:has-files={paths.length > 0}>
    {#if paths.length === 0}
      <div class="fz-empty">
        <button class="btn btn-au btn-lg" onclick={pickFiles}>Browse files…</button>
        <span class="fz-hint">or drag & drop here</span>
      </div>
    {:else}
      <div class="file-list">
        {#each paths as p, idx}
          <div class="file-item">
            <div class="fi-left">
              <span class="fi-name">{basename(p)}</span>
              <span class="fi-path">{p}</span>
            </div>
            <div class="fi-middle">
              {#if editingIdx === idx}
                <input
                  type="text"
                  class="fi-input"
                  value={names[idx]?.replace(/\.txt$/, '') || ''}
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    if (target) {
                      updateName(idx, target.value);
                    }
                  }}
                  onblur={() => (editingIdx = null)}
                />
              {:else}
                <button
                  class="fi-name-btn"
                  onclick={() => (editingIdx = idx)}
                  title="Click to edit filename"
                >
                  {names[idx] || 'unnamed.txt'}
                </button>
              {/if}
            </div>
            <button class="btn btn-sm btn-r btn-ic" onclick={() => remove(p)}
              ><Icon name="close" size={16} /></button
            >
          </div>
        {/each}
        <button class="btn btn-sm" onclick={pickFiles} style="margin-top:.3rem">Add more…</button>
      </div>
    {/if}
  </div>

  {#if paths.length > 0}
    <div style="margin-top:.7rem">
      <button class="btn btn-g" onclick={run} disabled={running}>
        {running ? 'Importing…' : `Import ${paths.length} file${paths.length > 1 ? 's' : ''}`}
      </button>
    </div>
  {/if}

  {#if result}
    <div class="result" class:has-err={result.errors > 0}>
      <div class="res-title">
        {result.errors ? '⚠ Import complete (with errors)' : '✓ Import complete'}
      </div>
      <div class="res-grid">
        <span>Words</span><strong>{result.words}</strong>
        <span>Definitions</span><strong>{result.definitions}</strong>
        <span>Events</span><strong>{result.events}</strong>
        <span>Types</span><strong>{result.types}</strong>
        <span>Authors</span><strong>{result.authors}</strong>
        <span>Errors</span><strong class:err={result.errors > 0}>{result.errors}</strong>
      </div>
      {#if result.messages.length > 0}
        <div class="res-log">
          {#each result.messages as m}<div>{m}</div>{/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .ip {
    max-width: 560px;
  }

  .github-section {
    margin-bottom: 1rem;
    padding: 0.8rem;
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .github-input-group {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .github-url-input {
    flex: 1;
    padding: 0.4rem 0.6rem;
    background: var(--surf);
    border: 1px solid var(--border2);
    border-radius: 4px;
    color: var(--text);
    font-size: 0.8rem;
  }

  .github-url-input:focus {
    outline: none;
    border-color: var(--gold);
    box-shadow: 0 0 0 2px var(--gold-t);
  }

  .github-url-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .github-help {
    font-size: 0.65rem;
    color: var(--text3);
    margin: 0;
  }

  .ip-help {
    font-size: 0.72rem;
    color: var(--text2);
    margin-bottom: 0.7rem;
    line-height: 1.65;
  }
  .ip-help code {
    color: var(--gold);
    background: var(--gold-g);
    padding: 0 3px;
    border-radius: 2px;
    font-size: 0.88em;
  }
  .file-zone {
    border: 1px dashed var(--border2);
    border-radius: 6px;
    padding: 0.8rem;
    margin-bottom: 0.3rem;
    min-height: 80px;
  }
  .fz-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0;
  }
  .fz-hint {
    font-size: 0.65rem;
    color: var(--text3);
  }
  .file-list {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
  }
  .file-item {
    display: flex;
    align-items: center;
    gap: 0.38rem;
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.22rem 0.45rem;
  }
  .fi-left {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    flex: 1;
    min-width: 0;
  }
  .fi-name {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--gold);
    white-space: nowrap;
  }
  .fi-path {
    font-size: 0.57rem;
    color: var(--text3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fi-middle {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    min-width: 120px;
  }
  .fi-input {
    flex: 1;
    padding: 0.2rem 0.3rem;
    font-size: 0.65rem;
    background: var(--surf);
    border: 1px solid var(--gold);
    border-radius: 3px;
    color: var(--gold);
  }
  .fi-input:focus {
    outline: none;
    box-shadow: 0 0 0 2px var(--gold-t);
  }
  .fi-name-btn {
    flex: 1;
    padding: 0.2rem 0.3rem;
    font-size: 0.65rem;
    background: rgba(218, 165, 32, 0.1);
    border: 1px solid var(--gold-d);
    border-radius: 3px;
    color: var(--gold);
    cursor: pointer;
    white-space: nowrap;
    text-align: left;
  }
  .fi-name-btn:hover {
    background: rgba(218, 165, 32, 0.15);
  }
  .result {
    margin-top: 0.8rem;
    background: var(--surf2);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 0.7rem 0.85rem;
  }
  .result.has-err {
    border-color: var(--red-d);
  }
  .res-title {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--green);
    margin-bottom: 0.4rem;
  }
  .has-err .res-title {
    color: var(--red);
  }
  .res-grid {
    display: grid;
    grid-template-columns: max-content auto;
    gap: 0.08rem 0.6rem;
    font-size: 0.7rem;
    color: var(--text2);
    margin-bottom: 0.4rem;
  }
  .res-grid strong {
    color: var(--text);
    font-weight: 600;
  }
  .res-grid strong.err {
    color: var(--red);
  }
  .res-log {
    font-size: 0.65rem;
    color: var(--text3);
    background: var(--surf);
    border-radius: 3px;
    padding: 0.3rem 0.4rem;
    max-height: 120px;
    overflow-y: auto;
    line-height: 1.6;
  }
</style>
