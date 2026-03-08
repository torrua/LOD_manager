<script lang="ts">
  import { app, saveType, deleteType } from '../store.svelte';
  let editing = $state<number | null>(null);
  let creating = $state(false);
  let form = $state({ name: '', type_x: '', group_: '' });

  function startEdit(id: number) {
    const t = app.types.find((x) => x.id === id)!;
    form = { name: t.name, type_x: t.type_x || '', group_: t.group_ || '' };
    editing = id;
    creating = false;
  }
  function startNew() {
    form = { name: '', type_x: '', group_: '' };
    creating = true;
    editing = null;
  }
  function cancel() {
    editing = null;
    creating = false;
  }
  async function submit() {
    if (!form.name.trim()) return;
    await saveType(editing, {
      name: form.name,
      type_x: form.type_x || null,
      group_: form.group_ || null,
    });
    editing = null;
    creating = false;
  }
  // issue #7: respond to topbar + New signal
  $effect(() => {
    if (app.newSignal > 0 && app.tab === 'types') startNew();
  });
</script>

<div>
  <!-- issue #7: no duplicate + New here; only topbar triggers it -->
  <div class="sec" style="margin-bottom:.5rem;margin-top:.1rem">
    <span class="sec-title">Word Types ({app.types.length})</span>
  </div>

  {#if creating}{@render typeForm('New Type')}{/if}

  <table class="data-table">
    <thead
      ><tr>
        <th>Name</th><th>Abbrev</th><th>Group</th>
        <th style="text-align:right">Words</th>
        {#if !app.readonly}<th></th>{/if}
      </tr></thead
    >
    <tbody>
      {#each app.types as t}
        {#if editing === t.id}
          <tr><td colspan="5">{@render typeForm(`Edit: ${t.name}`)}</td></tr>
        {:else}
          <tr>
            <td><span class="td-name">{t.name}</span></td>
            <td><span class="td-sub">{t.type_x || '—'}</span></td>
            <td><span class="td-sub">{t.group_ || '—'}</span></td>
            <td style="text-align:right"><span class="td-sub">{t.word_count}</span></td>
            {#if !app.readonly}
              <td>
                <div class="row-acts edit-only">
                  <button class="btn btn-icon btn-sm btn-ghost" onclick={() => startEdit(t.id)}
                    >✎</button
                  >
                  <button
                    class="btn btn-icon btn-sm btn-ghost btn-r"
                    onclick={() => deleteType(t.id)}>🗑</button
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

{#snippet typeForm(title: string)}
  <div class="inline-form">
    <div class="if-title">{title}</div>
    <div class="form-row">
      <div class="fg">
        <label for="tp-name">Name *</label><input
          id="tp-name"
          class="fi"
          bind:value={form.name}
          placeholder="C-Prim"
        />
      </div>
      <div class="fg">
        <label for="tp-abbr">Abbreviation</label><input
          id="tp-abbr"
          class="fi"
          bind:value={form.type_x}
          placeholder="CP"
        />
      </div>
      <div class="fg">
        <label for="tp-group">Group</label><input
          id="tp-group"
          class="fi"
          bind:value={form.group_}
          placeholder="Primitive"
        />
      </div>
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
