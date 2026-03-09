<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, saveAuthor, deleteAuthor } from '../store.svelte';
  let editing = $state<number | null>(null);
  let creating = $state(false);
  let form = $state({ initials: '', full_name: '', notes: '' });

  function startEdit(id: number) {
    const a = app.authors.find((x) => x.id === id);
    if (!a) return;
    form = { initials: a.initials, full_name: a.full_name || '', notes: a.notes || '' };
    editing = id;
    creating = false;
  }
  function startNew() {
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
    if (app.newSignal > 0 && app.tab === 'authors') startNew();
  });
</script>

<div>
  <!-- issue #7: no duplicate + New here -->
  <div class="sec" style="margin-bottom:.5rem;margin-top:.1rem">
    <span class="sec-title">Authors ({app.authors.length})</span>
  </div>

  {#if creating}{@render authorForm('New Author')}{/if}

  <table class="data-table">
    <thead
      ><tr>
        <th>Initials</th><th>Full name</th><th>Notes</th>
        {#if !app.readonly}<th></th>{/if}
      </tr></thead
    >
    <tbody>
      {#each app.authors as a}
        {#if editing === a.id}
          <tr><td colspan="4">{@render authorForm(`Edit: ${a.initials}`)}</td></tr>
        {:else}
          <tr>
            <td><span class="td-name">{a.initials}</span></td>
            <td><span class="td-sub">{a.full_name || '—'}</span></td>
            <td><span class="td-sub">{a.notes || ''}</span></td>
            {#if !app.readonly}
              <td>
                <div class="row-acts edit-only">
                  <button class="btn btn-icon btn-sm btn-ghost" onclick={() => startEdit(a.id)}
                    >✎</button
                  >
                  <button
                    class="btn btn-icon btn-sm btn-ghost btn-r"
                    onclick={() => deleteAuthor(a.id)}><Icon name="delete" size={14} /></button
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
        placeholder="Optional notes about this author…"
      ></textarea>
    </div>
    <div class="form-actions">
      <button class="btn btn-g btn-sm" onclick={submit}>Save</button>
      <button class="btn btn-sm" onclick={cancel}>Cancel</button>
    </div>
  </div>
{/snippet}

<style>
  /* .inline-form and .if-title are global classes */
</style>
