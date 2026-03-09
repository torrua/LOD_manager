<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, saveWord } from '../store.svelte';

  const w = app.curWord;
  const form = $state({
    name: w?.name ?? '',
    type_name: w?.type_name ?? '',
    source: w?.source ?? '',
    year: w?.year ?? '',
    rank: w?.rank ?? '',
    match_: w?.match_ ?? '',
    origin: w?.origin ?? '',
    origin_x: w?.origin_x ?? '',
    notes: w?.notes ?? '',
    event_start: w?.event_start_name ?? '',
    event_end: w?.event_end_name ?? '',
    affixes: [...(w?.affixes ?? [])],
    spellings: [...(w?.spellings ?? [])],
  });
  let newAffix = $state('');
  let newSpell = $state('');
  let saving = $state(false);
  let errors = $state<Record<string, string>>({});

  function validate() {
    errors = {};
    if (!form.name.trim()) errors.name = 'Required';
    return Object.keys(errors).length === 0;
  }

  async function submit() {
    if (!validate() || saving) return;
    saving = true;
    try {
      await saveWord(w?.id ?? null, {
        ...form,
        type_name: form.type_name || null,
        source: form.source || null,
        year: form.year || null,
        rank: form.rank || null,
        match_: form.match_ || null,
        origin: form.origin || null,
        origin_x: form.origin_x || null,
        notes: form.notes || null,
        event_start: form.event_start || null,
        event_end: form.event_end || null,
      });
    } finally {
      saving = false;
    }
  }

  function cancel() {
    app.editing = false;
    app.panel = w ? 'word' : 'welcome';
    if (!w) app.mobileShowList = true;
  }

  function addAffix() {
    const a = newAffix.trim().toLowerCase();
    if (a && !form.affixes.includes(a)) form.affixes = [...form.affixes, a];
    newAffix = '';
  }
  function removeAffix(a: string) {
    form.affixes = form.affixes.filter((x) => x !== a);
  }
  function affixKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      addAffix();
    }
  }

  function addSpell() {
    const s = newSpell.trim();
    if (s && !form.spellings.includes(s)) form.spellings = [...form.spellings, s];
    newSpell = '';
  }
  function removeSpell(s: string) {
    form.spellings = form.spellings.filter((x) => x !== s);
  }
  function spellKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      addSpell();
    }
  }
</script>

<div class="wf">
  <div class="wf-title">{w ? `Edit: ${w.name}` : 'New Word'}</div>

  <!-- Row 1: word name + type -->
  <div class="form-row fr-2">
    <div class="fg">
      <label for="wf-name">Word *</label>
      <input
        id="wf-name"
        class="fi"
        class:err={errors.name}
        bind:value={form.name}
        placeholder="e.g. kliri"
        autocomplete="off"
      />
      {#if errors.name}<span class="errtxt">{errors.name}</span>{/if}
    </div>
    <div class="fg">
      <label for="wf-type">Type</label>
      <select id="wf-type" class="fsel" bind:value={form.type_name}>
        <option value="">— none —</option>
        {#each app.types as t}
          <option value={t.name}>{t.name}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Row 2: source / year / rank / match -->
  <div class="form-row fr-4">
    <div class="fg">
      <label for="wf-source">Source</label>
      <input id="wf-source" class="fi" bind:value={form.source} placeholder="JCB, L4…" />
    </div>
    <div class="fg">
      <label for="wf-year">Year</label>
      <input id="wf-year" class="fi" bind:value={form.year} placeholder="'75" />
    </div>
    <div class="fg">
      <label for="wf-rank">Rank</label>
      <input id="wf-rank" class="fi" bind:value={form.rank} placeholder="1.0" />
    </div>
    <div class="fg">
      <label for="wf-match">Match %</label>
      <input id="wf-match" class="fi" bind:value={form.match_} placeholder="49%" />
    </div>
  </div>

  <!-- Row 3: event start + event end -->
  <div class="form-row fr-2">
    <div class="fg">
      <label for="wf-evstart">Event Start</label>
      <select id="wf-evstart" class="fsel" bind:value={form.event_start}>
        <option value="">— none —</option>
        {#each app.events as e}<option value={e.name}>{e.name}</option>{/each}
      </select>
    </div>
    <div class="fg">
      <label for="wf-evend">Event End</label>
      <select id="wf-evend" class="fsel" bind:value={form.event_end}>
        <option value="">— none —</option>
        {#each app.events as e}<option value={e.name}>{e.name}</option>{/each}
      </select>
    </div>
  </div>

  <!-- Row 4: origin -->
  <div class="form-row fr-origin">
    <div class="fg" style="flex:3">
      <label for="wf-origin">Origin</label>
      <input
        id="wf-origin"
        class="fi"
        bind:value={form.origin}
        placeholder="3/4E clear | 4/5J kirei…"
      />
    </div>
    <div class="fg" style="flex:2">
      <label for="wf-originx">Origin (=)</label>
      <input id="wf-originx" class="fi" bind:value={form.origin_x} />
    </div>
  </div>

  <!-- Row 5: affixes + spellings side by side -->
  <div class="form-row fr-2" style="align-items:start">
    <!-- Affixes -->
    <div class="fg">
      <label for="wf-affix-inp">Affixes</label>
      <div class="tag-field">
        {#each form.affixes as a}
          <span class="badge bd-afx"
            >{a}
            <button class="tag-rm" onclick={() => removeAffix(a)}
              ><Icon name="close" size={11} /></button
            >
          </span>
        {/each}
        <input
          id="wf-affix-inp"
          class="fi tag-inp"
          bind:value={newAffix}
          placeholder="add…"
          onkeydown={affixKeydown}
        />
        <button class="btn btn-add" onclick={addAffix} title="Add affix">+</button>
      </div>
    </div>
    <!-- Spellings -->
    <div class="fg">
      <label for="wf-spell-inp">Alternate Spellings</label>
      <div class="tag-field">
        {#each form.spellings as s}
          <span class="badge bd-spell"
            >{s}
            <button class="tag-rm" onclick={() => removeSpell(s)}
              ><Icon name="close" size={11} /></button
            >
          </span>
        {/each}
        <input
          id="wf-spell-inp"
          class="fi tag-inp"
          bind:value={newSpell}
          placeholder="add…"
          onkeydown={spellKeydown}
        />
        <button class="btn btn-add" onclick={addSpell} title="Add spelling">+</button>
      </div>
    </div>
  </div>

  <!-- Notes -->
  <div class="fg" style="margin-bottom:.5rem">
    <label for="wf-notes">Notes</label>
    <textarea id="wf-notes" class="fta" bind:value={form.notes} rows="3"></textarea>
  </div>

  <div class="form-actions">
    <button class="btn btn-g" onclick={submit} disabled={saving}>
      {saving ? 'Saving…' : w ? 'Save changes' : 'Create word'}
    </button>
    <button class="btn" onclick={cancel}>Cancel</button>
  </div>
</div>

<style>
  .wf {
    max-width: 640px;
  }
  .wf-title {
    font-size: var(--fs-md);
    font-weight: 700;
    color: var(--text);
    margin-bottom: 0.85rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border);
  }

  /* form-row variants */
  .fr-2 {
    grid-template-columns: 1fr 1fr;
  }
  .fr-4 {
    grid-template-columns: repeat(4, 1fr);
  }
  .fr-origin {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.44rem;
  }

  /* error */
  .fi.err {
    border-color: var(--red-d);
  }
  .errtxt {
    font-size: 0.58rem;
    color: var(--red);
    margin-top: 0.1rem;
  }

  /* Tag field: wraps chips + input + button */
  .tag-field {
    display: flex;
    flex-wrap: wrap;
    gap: 0.22rem;
    align-items: center;
    min-height: 28px; /* same height as .fi */
    padding: 0.2rem 0.3rem;
    background: var(--inp-bg);
    border: 1px solid var(--inp-border);
    border-radius: 4px;
    transition: border-color 140ms;
  }
  .tag-field:focus-within {
    border-color: var(--gold-d);
  }

  .tag-inp {
    flex: 1;
    min-width: 40px;
    height: 20px;
    background: transparent;
    border: none;
    outline: none;
    font-family: inherit;
    font-size: 0.68rem;
    color: var(--text);
    padding: 0;
  }
  .tag-inp::placeholder {
    color: var(--text3);
  }

  /* + button inside tag field — same visual weight as .btn-sm but square */
  .btn-add {
    width: 20px;
    height: 20px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    font-weight: 700;
    line-height: 1;
    border-radius: 3px;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text3);
    cursor: pointer;
    font-family: inherit;
    transition: all 120ms;
    flex-shrink: 0;
  }
  .btn-add:hover {
    background: var(--green-g, rgba(90, 160, 80, 0.12));
    border-color: var(--green-d, color-mix(in srgb, var(--green) 70%, black));
    color: var(--green, var(--green));
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.1rem;
    font-size: var(--fs-label);
    padding: 2px 5px;
    border-radius: var(--r-sm);
    line-height: 1.4;
  }
  .bd-afx {
    background: rgba(106, 170, 112, 0.12);
    color: var(--green, var(--green));
    border: 1px solid rgba(106, 170, 112, 0.25);
  }
  .bd-spell {
    background: rgba(106, 154, 200, 0.12);
    color: var(--blue);
    border: 1px solid rgba(106, 154, 200, 0.25);
  }
  .tag-rm {
    background: none;
    border: none;
    color: currentColor;
    cursor: pointer;
    padding: 0;
    font-size: 0.72rem;
    opacity: 0.6;
    line-height: 1;
  }
  .tag-rm:hover {
    opacity: 1;
  }

  /* responsive: stack 4-col to 2-col on narrow */
  @media (max-width: 520px) {
    .fr-4 {
      grid-template-columns: 1fr 1fr;
    }
    .fr-2 {
      grid-template-columns: 1fr;
    }
    .fr-origin {
      flex-direction: column;
    }
  }
</style>
