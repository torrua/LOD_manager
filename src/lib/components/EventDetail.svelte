<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { app, deleteEvent, selectWord } from '../store.svelte';
  import type { EventItem } from '../../types';

  let { event }: { event: EventItem } = $props();

  let confirmDel   = $state(false);
  let wordsAdded   = $state<string[]>([]);
  let wordsRemoved = $state<string[]>([]);
  let loadingWords = $state(false);
  // collapsed state: true = collapsed
  let addedOpen    = $state(true);
  let removedOpen  = $state(true);

  $effect(() => {
    const id = event.id;
    wordsAdded = []; wordsRemoved = []; loadingWords = true;
    addedOpen = true; removedOpen = true;
    invoke<[string[], string[]]>('get_event_words', { eventId: id })
      .then(([a, r]) => { wordsAdded = a; wordsRemoved = r; })
      .catch(() => {})
      .finally(() => { loadingWords = false; });
  });

  function clickWord(name: string) {
    const w = app.words.find(x => x.name === name);
    if (w) selectWord(w.id);
  }
</script>

<div class="ed">

  <!-- ── Header ── -->
  <div class="ed-header">
    <h1 class="ed-name">{event.name}</h1>
    {#if !app.readonly}
      <div class="ed-acts edit-only">
        <button class="btn btn-au btn-sm"
          onclick={() => { app.panel = 'event-form'; app.editing = true; }}>✎ Edit</button>
        <button class="btn btn-r btn-sm btn-icon"
          onclick={() => confirmDel = true}>🗑</button>
      </div>
    {/if}
  </div>

  <!-- ── Meta chips ── -->
  <div class="ed-meta">
    {#if event.date}
      <div class="mc">
        <span class="mc-lbl">Date</span>
        <span class="mc-val">{event.date}</span>
      </div>
    {/if}
    {#if event.annotation}
      <div class="mc">
        <span class="mc-lbl">Code</span>
        <span class="mc-val mono">{event.annotation}</span>
      </div>
    {/if}
    {#if event.suffix}
      <div class="mc">
        <span class="mc-lbl">Suffix</span>
        <span class="mc-val mono">{event.suffix}</span>
      </div>
    {/if}
  </div>

  <!-- ── Notes ── -->
  {#if event.notes}
    <section class="ed-section">
      <div class="sec-hd plain">
        <span class="sec-title">Notes</span>
      </div>
      <p class="ed-notes">{event.notes}</p>
    </section>
  {/if}

  <!-- ── Words ── -->
  {#if loadingWords}
    <div class="ed-loading">Loading words…</div>
  {:else}

    {#if wordsAdded.length > 0}
      <section class="ed-section">
        <button class="sec-hd" onclick={() => addedOpen = !addedOpen}>
          <span class="sec-title add">Words added</span>
          <span class="sec-cnt">{wordsAdded.length}</span>
          <span class="sec-arrow" class:open={addedOpen}>›</span>
        </button>
        {#if addedOpen}
          <div class="word-chips">
            {#each wordsAdded as name}
              <button class="wchip" onclick={() => clickWord(name)}>{name}</button>
            {/each}
          </div>
        {/if}
      </section>
    {/if}

    {#if wordsRemoved.length > 0}
      <section class="ed-section">
        <button class="sec-hd" onclick={() => removedOpen = !removedOpen}>
          <span class="sec-title rem">Words removed</span>
          <span class="sec-cnt">{wordsRemoved.length}</span>
          <span class="sec-arrow" class:open={removedOpen}>›</span>
        </button>
        {#if removedOpen}
          <div class="word-chips">
            {#each wordsRemoved as name}
              <button class="wchip rem" onclick={() => clickWord(name)}>{name}</button>
            {/each}
          </div>
        {/if}
      </section>
    {/if}

    {#if wordsAdded.length === 0 && wordsRemoved.length === 0}
      <div class="ed-empty">No word changes recorded for this event.</div>
    {/if}
  {/if}
</div>

<!-- ── Delete confirm ── -->
{#if confirmDel}
  <div class="modal-overlay" onclick={() => confirmDel = false}
    onkeydown={(e) => e.key === "Escape" && (confirmDel = false)}
    role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" onclick={e => e.stopPropagation()}>
      <div class="modal-title">Delete "{event.name}"?</div>
      <p class="modal-body">This cannot be undone.</p>
      <div class="modal-actions">
        <button class="btn" onclick={() => confirmDel = false}>Cancel</button>
        <button class="btn btn-r"
          onclick={() => { confirmDel = false; deleteEvent(event.id); }}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .ed-header{
    display:flex;align-items:flex-start;justify-content:space-between;
    gap:.6rem;margin-bottom:.7rem;
  }
  .ed-name{font-size:1.35rem;font-weight:700;color:var(--text);margin:0;line-height:1.2}
  .ed-acts{display:flex;gap:.22rem;flex-shrink:0;padding-top:.1rem}

  .ed-meta{display:flex;flex-wrap:wrap;gap:.3rem;margin-bottom:.75rem}
  .mc{
    display:flex;flex-direction:column;gap:.1rem;
    padding:.28rem .56rem;background:var(--surf2);
    border:1px solid var(--border);border-radius:var(--r-md);min-width:60px;
  }
  /* mc-lbl, mc-val are global tokens from App.svelte */
  .mc-val.mono{font-family:'ui-monospace','Cascadia Code','Consolas',monospace}

  .ed-section{margin-bottom:.75rem}

  /* section header — button for collapsible, div for plain */
  .sec-hd{
    width:100%;display:flex;align-items:center;gap:.35rem;
    margin-bottom:.38rem;
    background:none;border:none;padding:0;
    cursor:pointer;text-align:left;font:inherit;
    -webkit-tap-highlight-color:transparent;
  }
  .sec-hd.plain{cursor:default}
  .sec-hd:not(.plain):hover .sec-title{opacity:.8}

  .sec-title{
    font-size:var(--fs-label);font-weight:700;text-transform:uppercase;
    letter-spacing:.1em;color:var(--text3);
  }
  .sec-title.add{color:var(--green)}
  .sec-title.rem{color:var(--red)}

  /* line after title */
  .sec-hd::after{
    content:'';flex:1;height:1px;background:var(--border);
  }

  .sec-cnt{
    font-size:.55rem;font-weight:700;
    padding:2px 6px;border-radius:var(--r-sm);
    background:var(--surf2);color:var(--text2);border:1px solid var(--border);
    flex-shrink:0;
  }
  .sec-arrow{
    font-size:.8rem;color:var(--text3);flex-shrink:0;
    transform:rotate(90deg);transition:transform 180ms ease;display:inline-block;
    line-height:1;margin-right:.1rem;
  }
  .sec-arrow.open{transform:rotate(-90deg)}

  .ed-notes{
    font-size:var(--fs-base);color:var(--text2);line-height:1.65;margin:0;
    padding:.48rem .6rem;background:var(--surf2);
    border-radius:var(--r-md);border:1px solid var(--border);
  }

  .word-chips{display:flex;flex-wrap:wrap;gap:.28rem;padding:.1rem 0}
  .wchip{
    padding:.22rem .52rem;
    font-family:var(--mono,'ui-monospace',monospace);
    font-size:.68rem;font-weight:500;color:var(--teal);
    background:rgba(86,186,180,.07);border:1px solid rgba(86,186,180,.2);
    border-radius:var(--r-md);cursor:pointer;transition:all 120ms;
    -webkit-tap-highlight-color:transparent;
  }
  .wchip:hover{background:rgba(86,186,180,.18);border-color:rgba(86,186,180,.45)}
  .wchip.rem{color:var(--text3);background:rgba(128,128,128,.06);border-color:rgba(128,128,128,.15)}
  .wchip.rem:hover{background:rgba(196,68,68,.08);border-color:rgba(196,68,68,.25);color:var(--red,#c44)}

  .ed-empty{font-size:var(--fs-sm);color:var(--text3);padding:.6rem 0;font-style:italic}
  .ed-loading{font-size:var(--fs-xs);color:var(--text3);padding:.6rem 0}
</style>
