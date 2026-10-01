<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, saveAuthor, deleteAuthor } from '../store.svelte';
  let editing = $state<number | null>(null);
  let creating = $state(false);
  let form = $state({ initials: '', full_name: '', notes: '' });

  function startEdit(id: number) {
    if (app.readonly) return;
    const a = app.authors.find((x) => x.id === id);
    if (!a) return;
    form = { initials: a.initials, full_name: a.full_name || '', notes: a.notes || '' };
    editing = id;
    creating = false;
  }
  function startNew() {
    if (app.readonly) return;
    form = { initials: '', full_name: '', notes: '' };
    creating = true;
    editing = null;
  }
  function cancel() {
    editing = null;
    creating = false;
  }
  async function submit() {
    if (!form.initials.trim()) return;
    await saveAuthor(editing, {
      initials: form.initials,
      full_name: form.full_name || null,
      notes: form.notes || null,
    });
    editing = null;
    creating = false;
  }
  // issue #7: respond to topbar + New signal
  $effect(() => {
    if (app.newSignal > 0 && app.tab === 'authors') {
      startNew();
      app.newSignal = 0; // Reset signal after processing
    }
  });
</script>

<div>
  <!-- issue #7: no duplicate + New here -->
  <div class="sec" style="margin-bottom:.5rem;margin-top:.1rem">
    <span class="sec-title">Authors ({app.authors.length})</span>
  </div>

  {#if creating}{@render authorForm('New Author')}{/if}

  <table class="data-table authors-table">
    <colgroup>
      <col class="col-initials" />
      <col class="col-name" />
      <col class="col-notes" />
      {#if !app.readonly}<col class="col-acts" />{/if}
    </colgroup>
    <thead>
      <tr>
        <th class="col-initials">Initials</th>
        <th class="col-name">Full name</th>
        <th class="col-notes">Notes</th>
        {#if !app.readonly}<th class="col-acts"></th>{/if}
      </tr>
    </thead>
    <tbody>
      {#each app.authors as a}
        {#if editing === a.id}
          <tr
            ><td colspan={app.readonly ? 3 : 4}>{@render authorForm(`Edit: ${a.initials}`)}</td></tr
          >
        {:else}
          <tr>
            <td class="col-initials"><span class="td-name">{a.initials}</span></td>
            <td class="col-name"><span class="td-sub">{a.full_name || '—'}</span></td>
            <td class="col-notes"><span class="td-sub td-notes">{a.notes || ''}</span></td>
            {#if !app.readonly}
              <td class="col-acts">
                <div class="row-acts edit-only">
                  <button class="btn btn-ic btn-ghost" onclick={() => startEdit(a.id)}
                    ><Icon name="edit" size={16} /></button
                  >
                  <button class="btn btn-ic btn-ghost btn-r" onclick={() => deleteAuthor(a.id)}
                    ><Icon name="delete" size={16} /></button
                  >
                </div>
              </td>
            {/if}
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
</div>

{#snippet authorForm(title: string)}
  <div class="inline-form">
    <div class="if-title">{title}</div>
    <div class="form-row">
      <div class="fg">
        <label for="ap-initials">Initials *</label><input
          id="ap-initials"
          class="fi"
          bind:value={form.initials}
          placeholder="JCB"
        />
      </div>
      <div class="fg">
        <label for="ap-fname">Full name</label><input
          id="ap-fname"
          class="fi"
          bind:value={form.full_name}
          placeholder="James Cooke Brown"
        />
      </div>
    </div>
    <div class="fg" style="margin-bottom:.42rem">
      <label for="ap-notes">Notes / comments</label>
      <textarea
        id="ap-notes"
        class="fta"
        bind:value={form.notes}
        rows="2"
        placeholder="Optional notes about this author…"></textarea>
    </div>
    <div class="form-actions">
      <button class="btn btn-g btn-sm" onclick={submit}>Save</button>
      <button class="btn btn-sm" onclick={cancel}>Cancel</button>
    </div>
  </div>
{/snippet}

<style>
  /* .inline-form and .if-title are global classes */
  .authors-table {
    table-layout: fixed;
    width: 100%;
    max-width: 100%;
  }
  .col-initials {
    width: 68px;
  }
  .col-name {
    width: 32%;
  }
  .col-notes {
    width: auto;
  }
  .col-acts {
    width: 56px;
    text-align: right;
  }
  :global(.authors-table td) {
    overflow-wrap: anywhere;
    word-break: break-word;
    vertical-align: top;
  }
  :global(.authors-table .td-notes) {
    display: inline-block;
    max-width: 100%;
    overflow-wrap: anywhere;
    word-break: break-word;
    white-space: pre-wrap;
    line-height: 1.4;
  }
  @media (max-width: 640px) {
    .col-initials {
      width: 48px;
    }
    .col-name {
      width: 34%;
    }
    .col-acts {
      width: 48px;
    }
  }
</style>
