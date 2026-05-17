<!--
  ToastHost — bottom-right transient one-liner driven by `app.toast`.
  Auto-dismisses (store-side) after ~2.2 s; clickable to dismiss early.
  Intentionally minimal: no queue, no stacking — newest replaces older
  so the user sees the latest action, not a backlog.
-->
<script lang="ts">
  import { app } from "$lib/stores/app.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
</script>

{#if app.toast}
  {@const t = app.toast}
  <button
    type="button"
    class="toast"
    class:err={t.kind === "err"}
    onclick={() => app.dismissToast()}
    aria-live="polite"
  >
    <span class="toast-icon">
      <Icon d={t.kind === "ok" ? IC.check : IC.x} size={12} />
    </span>
    <span class="toast-text">{t.text}</span>
  </button>
{/if}

<style>
  .toast {
    position: fixed;
    right: 14px;
    bottom: 14px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 13px 9px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    color: var(--fg);
    font-family: var(--ui);
    font-size: 12px;
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 0 1px rgba(245, 158, 11, 0.06);
    cursor: pointer;
    z-index: 1000;
    /* Slide-in from bottom — Svelte handles the mount; this is a CSS
       animation rather than a transition so it only runs once. */
    animation: toast-in 180ms ease-out both;
  }
  .toast:hover {
    border-color: var(--accent-bd);
  }
  .toast-icon {
    display: inline-flex;
    color: var(--ok);
  }
  .toast.err .toast-icon {
    color: var(--err);
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
