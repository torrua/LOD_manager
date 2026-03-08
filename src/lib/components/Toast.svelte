<script lang="ts">
  import { app } from '../store.svelte';
</script>

{#if app.toast}
  <div class="toast"
    class:ok={app.toast.kind==='ok'}
    class:err={app.toast.kind==='err'}
    class:info={app.toast.kind==='info'}>
    {app.toast.msg}
  </div>
{/if}

<style>
  /* sit above bottom-bar on compact, right-aligned otherwise */
  .toast{
    position:fixed;bottom:calc(var(--bb-h,0px) + .75rem);right:1rem;
    z-index:300;
    padding:.38rem .85rem;border-radius:4px;font-size:.72rem;font-weight:600;
    border:1px solid var(--border);background:var(--surf);color:var(--text);
    box-shadow:0 4px 16px var(--shd);
    animation:toast-in 180ms ease;
    pointer-events:none;max-width:280px;
  }
  .toast.ok {color:var(--green);border-color:var(--green-d);background:var(--green-g)}
  .toast.err{color:var(--red);border-color:var(--red-d);background:var(--red-g)}
  @keyframes toast-in{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:translateY(0)}}
  /* compact: centre horizontally above bottom bar */
  @media(max-width:520px){
    .toast{left:1rem;right:1rem;text-align:center;bottom:calc(var(--bb-h,48px) + .5rem)}
  }
</style>
