<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { importFiles, toast } from '../store.svelte';
  import type { ImportResult } from '../../types';

  let paths = $state<string[]>([]);
  let names = $state<string[]>([]); // display filenames (for Android content:// URIs)
  let result = $state<ImportResult | null>(null);
  let running = $state(false);

  async function pickFiles() {
    const selected = await open({
      multiple: true,
      title: 'Select LOD text files',
      filters: [{ name: 'Text', extensions: ['txt'] }],
    });
    if (selected) {
      paths = Array.isArray(selected) ? selected : [selected];
      // Extract display names — on Android these may just be the URI segment
      names = paths.map((p) => {
        const decoded = decodeURIComponent(p);
        // For content URIs, try to extract filename from URI or use fallback
        if (p.startsWith('content://')) {
          // Try different strategies to extract filename from content URI
          const uriParts = decoded.split('/');
          const lastPart = uriParts[uriParts.length - 1];
          // Remove query parameters and fragments
          const cleanName = lastPart?.split('?')[0]?.split('#')[0] || '';
          // If it looks like a filename, use it; otherwise use generic name
          if (cleanName && (cleanName.includes('.') || cleanName.length > 10)) {
            return cleanName;
          }
          // Fallback: try to get filename from document ID or use generic
          const docId = uriParts.find((part) => part.includes('document'))?.split('document/')[1];
          return docId ? `file_${docId}.txt` : `android_file_${Date.now()}.txt`;
        }
        // For regular file paths
        return decoded.split(/[/]/).pop()?.split('?')[0] || p;
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

  <div class="file-zone" class:has-files={paths.length > 0}>
    {#if paths.length === 0}
      <div class="fz-empty">
        <button class="btn btn-au btn-lg" onclick={pickFiles}>Browse files…</button>
        <span class="fz-hint">or drag & drop here</span>
      </div>
    {:else}
      <div class="file-list">
        {#each paths as p}
          <div class="file-item">
            <span class="fi-name">{basename(p)}</span>
            <span class="fi-path">{p}</span>
            <button class="btn btn-sm btn-r" onclick={() => remove(p)}>×</button>
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
  .fi-name {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--gold);
    white-space: nowrap;
  }
  .fi-path {
    font-size: 0.57rem;
    color: var(--text3);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
