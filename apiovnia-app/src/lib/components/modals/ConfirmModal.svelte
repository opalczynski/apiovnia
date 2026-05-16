<!--
  ConfirmModal — yes/no with optional danger styling (red confirm button for
  destructive actions like delete).
-->
<script lang="ts">
  import type { ConfirmOptions } from "$lib/stores/dialogs.svelte";

  type Props = {
    opts: ConfirmOptions;
    onResolve: (value: boolean) => void;
  };

  const { opts, onResolve }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let confirmBtn: HTMLButtonElement | undefined = $state();

  $effect(() => {
    if (!dialogEl) return;
    dialogEl.showModal();
    queueMicrotask(() => confirmBtn?.focus());
  });

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onResolve(false);
  }
</script>

<dialog
  bind:this={dialogEl}
  class="modal"
  oncancel={(e) => {
    e.preventDefault();
    onResolve(false);
  }}
  onclick={onBackdropClick}
>
  <form method="dialog" class="card">
    <header class="head">
      <div class="title">{opts.title}</div>
      {#if opts.message}
        <div class="msg">{opts.message}</div>
      {/if}
    </header>

    <footer class="foot">
      <button type="button" class="ap-btn ghost" onclick={() => onResolve(false)}>
        {opts.cancelLabel ?? "Cancel"}
        <span class="ap-kbd">esc</span>
      </button>
      <div class="grow"></div>
      <button
        type="button"
        bind:this={confirmBtn}
        class="ap-btn"
        class:cta={!opts.danger}
        class:danger={opts.danger}
        onclick={() => onResolve(true)}
      >
        {opts.confirmLabel ?? "Confirm"}
        <span class="ap-kbd">↵</span>
      </button>
    </footer>
  </form>
</dialog>

<style>
  .modal {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    max-width: 100vw;
    max-height: 100vh;
    margin: 0;
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--fg);
    display: grid;
    place-items: center;
  }
  .modal:not([open]) {
    display: none;
  }
  .modal::backdrop {
    background: radial-gradient(
      circle at 50% 40%,
      rgba(0, 0, 0, 0.55),
      rgba(0, 0, 0, 0.85) 70%
    );
    backdrop-filter: blur(2px);
  }

  .card {
    width: 420px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow:
      0 24px 60px rgba(0, 0, 0, 0.55),
      0 0 0 1px rgba(245, 158, 11, 0.08);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--ui);
  }

  .head {
    padding: 18px 22px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .msg {
    font-size: 12.5px;
    color: var(--fg-muted);
    line-height: 1.5;
  }

  .foot {
    padding: 12px 22px 16px;
    display: flex;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-2);
  }
  .foot .ap-btn {
    height: 30px;
    padding: 0 14px;
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }

  /* Danger variant — red gradient button for destructive confirms. */
  .ap-btn.danger {
    background: linear-gradient(180deg, #f87171 0%, #dc2626 100%);
    border-color: #b91c1c;
    color: #fef2f2;
    font-weight: 600;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.2),
      0 1px 0 rgba(255, 255, 255, 0.18) inset;
  }
  .ap-btn.danger:hover {
    background: linear-gradient(180deg, #fca5a5 0%, #ef4444 100%);
  }
</style>
