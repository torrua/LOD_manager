<script lang="ts">
  import { app, saveEvent } from '../store.svelte';

  const ev = app.curEvent;
  const form = $state({
    name: ev?.name ?? '',
    date: ev?.date ?? '',
    annotation: ev?.annotation ?? '',
    suffix: ev?.suffix ?? '',
    notes: ev?.notes ?? '',
  });
  let saving = $state(false);

  async function submit() {
    if (!form.name.trim() || saving) return;
    saving = true;
    try {
      await saveEvent(ev?.id ?? null, {
        name: form.name,
        date: form.date || null,
        annotation: form.annotation || null,
        suffix: form.suffix || null,
        notes: form.notes || null,
      });
    } finally {
      saving = false;
    }
  }
  function cancel() {
    app.editing = false;
    app.panel = ev ? 'event' : 'welcome';
    if (!ev) app.mobileShowList = true;
  }
</script>

<div class="ef">
  <div class="ef-title">{ev ? `Edit: ${ev.name}` : 'New Event'}</div>

  <div class="form-row c2">
    <div class="fg">
      <label for="ef-name">Name *</label>
      <input id="ef-name" class="fi" bind:value={form.name} placeholder="Event name" />
    </div>
    <div class="fg">
      <label for="ef-date">Date</label>
      <input id="ef-date" class="fi" bind:value={form.date} placeholder="1/15/2016" />
    </div>
  </div>
  <div class="form-row c2">
    <div class="fg">
      <label for="ef-ann">Annotation (code)</label>
      <input id="ef-ann" class="fi" bind:value={form.annotation} placeholder="RDC" />
    </div>
    <div class="fg">
      <label for="ef-suffix">Filename suffix</label>
      <input id="ef-suffix" class="fi" bind:value={form.suffix} placeholder="" />
    </div>
  </div>
  <div class="fg" style="margin-bottom:.5rem">
    <label for="ef-notes">Notes / description</label>
    <textarea id="ef-notes" class="fta" bind:value={form.notes} rows="4"></textarea>
  </div>

  <div class="form-actions">
    <button class="btn btn-g" onclick={submit} disabled={saving}>
      {saving ? 'Saving…' : ev ? 'Save changes' : 'Create event'}
    </button>
    <button class="btn" onclick={cancel}>Cancel</button>
  </div>
</div>

<style>
  .ef {
    max-width: 480px;
  }
  .ef-title {
    font-size: var(--fs-md);
    font-weight: 700;
    color: var(--text);
    margin-bottom: 0.85rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border);
  }
  .form-row {
    display: grid;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .form-row.c2 {
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
  }
</style>
