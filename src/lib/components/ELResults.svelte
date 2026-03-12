<!-- eslint-disable svelte/no-at-html-tags -->
<script lang="ts">
  import { app, selectWord } from '../store.svelte';

  // Highlight «marked» tokens from FTS snippet, or the query in LIKE mode
  function renderSnippet(raw: string, query: string): string {
    if (!raw) return '';
    // FTS5 already wraps matches in «» — convert to <mark>
    let t = raw
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/«([^»]+)»/g, '<mark>$1</mark>');
    // If no marks yet (LIKE mode), highlight the query term
    if (!t.includes('<mark>') && query.trim()) {
      const q = query.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      t = t.replace(new RegExp(`(${q})`, 'gi'), '<mark>$1</mark>');
    }
    // Truncate long snippets: show first 120 chars
    const plain = t.replace(/<[^>]+>/g, '');
    if (plain.length > 120) {
      // Keep up to first <mark>
      const mi = t.indexOf('<mark>');
      const start = Math.max(0, mi - 30);
      t = `${(start > 0 ? '…' : '') + t.slice(start, start + 160)}…`;
    }
    return t;
  }

  const results = $derived(app.elResults);
  const hasMore = $derived(results.length >= 300);

  function handleSelect(wordId: number) {
    selectWord(wordId);
    requestAnimationFrame(() => {
      document.querySelector<HTMLElement>('.el-row.on')?.scrollIntoView({ block: 'nearest' });
    });
  }
</script>

<div class="el-root">
  {#if app.elSearching}
    <div class="el-status">Searching…</div>
  {:else if app.elQuery.trim() && results.length === 0}
    <div class="el-status empty">No results for "<span>{app.elQuery}</span>"</div>
  {:else if results.length > 0}
    {#each results as r (r.word_id)}
      <div
        class="el-row"
        class:on={app.curWord?.id === r.word_id}
        role="button"
        tabindex="0"
        onclick={() => handleSelect(r.word_id)}
        onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && handleSelect(r.word_id)}
      >
        <div class="el-top">
          <span class="el-name">{r.word_name}</span>
          {#if app.prefs.elShowDetails}
            {#if app.prefs.elShowType && r.type_name}
              <span class="el-type">{r.type_name}</span>
            {/if}
            {#if app.prefs.elShowGrammar && r.grammar}
              <span class="el-gram">({r.grammar})</span>
            {/if}
            {#if app.prefs.elShowCount && r.match_count > 1}
              <span class="el-cnt">{r.match_count}×</span>
            {/if}
          {/if}
        </div>

        {#if app.prefs.elShowDetails && app.prefs.elShowSnippet && r.snippet}
          <div class="el-snip">
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html renderSnippet(r.snippet, app.elQuery)}
          </div>
        {/if}
      </div>
    {/each}

    {#if hasMore}
      <div class="el-more">Showing first 300 results — refine your query</div>
    {/if}
  {/if}
</div>

<style>
  .el-root {
    display: flex;
    flex-direction: column;
    padding: 0.1rem 0;
  }
  .el-status {
    padding: 1.2rem 0.5rem;
    font-size: 0.72rem;
    color: var(--sb-text2);
    text-align: center;
  }
  .el-status.empty span {
    color: var(--gold);
  }

  .el-row {
    padding: 0.32rem 0.48rem 0.28rem 0.42rem;
    border-left: 2px solid transparent;
    cursor: pointer;
    transition:
      background 100ms,
      border-color 100ms;
    -webkit-tap-highlight-color: transparent;
  }
  .el-row:hover {
    background: var(--sb-hover);
  }
  .el-row.on {
    background: var(--gold-g);
    border-left-color: var(--gold);
  }

  .el-top {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
    flex-wrap: wrap;
    margin-bottom: 0.1rem;
  }
  .el-name {
    font-size: 0.73rem;
    font-weight: 600;
    color: var(--sb-text);
  }
  .el-row.on .el-name {
    color: var(--gold);
  }
  .el-type {
    font-size: 0.5rem;
    color: var(--sb-text2);
    background: rgba(128, 128, 128, 0.1);
    padding: 1px 4px;
    border-radius: 2px;
  }
  .el-gram {
    font-size: 0.55rem;
    color: var(--teal);
  }
  .el-cnt {
    font-size: 0.5rem;
    color: var(--blue);
    background: rgba(106, 154, 200, 0.1);
    border: 1px solid rgba(106, 154, 200, 0.25);
    padding: 0 3px;
    border-radius: 2px;
    margin-left: auto;
  }

  .el-snip {
    font-size: 0.63rem;
    color: var(--sb-text2);
    line-height: 1.5;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
  }
  /* highlighted match */
  :global(.el-snip mark) {
    background: transparent;
    color: var(--teal);
    font-weight: 600;
    font-style: italic;
  }

  .el-more {
    font-size: 0.6rem;
    color: var(--sb-text2);
    padding: 0.5rem 0.5rem 0.8rem;
    text-align: center;
    border-top: 1px solid var(--sb-border);
  }

  @media (max-width: 640px) {
    .el-row {
      padding: 0.4rem 0.5rem;
    }
    .el-name {
      font-size: 0.82rem;
    }
    .el-snip {
      font-size: 0.72rem;
    }
  }
</style>
