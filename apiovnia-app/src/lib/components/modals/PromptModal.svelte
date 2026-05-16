<!--
  PromptModal — text input with title/message. Enter commits, Esc cancels.
  Uses native <dialog> for focus trap + click-outside handling.
-->
<script lang="ts">
  import type { PromptOptions } from "$lib/stores/dialogs.svelte";

  type Props = {
    opts: PromptOptions;
    onResolve: (value: string | null) => void;
  };

  const { opts, onResolve }: Props = $props();

  let value = $state("");
  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();

  // Initialise default once and open the dialog on mount.
  $effect(() => {
    value = opts.defaultValue ?? "";
    if (!dialogEl) return;
    dialogEl.showModal();
    queueMicrotask(() => {
      inputEl?.focus();
      inputEl?.select();
    });
  });

  function commit() {
    const trimmed = value.trim();
    if (!trimmed) return;
    onResolve(trimmed);
  }

  function cancel() {
    onResolve(null);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commit();
    }
    // Esc handled by native <dialog> -> oncancel.
  }

  function onBackdropClick(e: MouseEvent) {
    // Clicks on the <dialog> itself (i.e. the backdrop, not the form inside)
    // mean "click outside" — treat as cancel.
    if (e.target === dialogEl) cancel();
  }
</script>

<dialog
  bind:this={dialogEl}
  class="modal"
  oncancel={(e) => {
    e.preventDefault();
    cancel();
  }}
  onclick={onBackdropClick}
>
  <form
    method="dialog"
    class="card"
    onsubmit={(e) => {
      e.preventDefault();
      commit();
    }}
  >
    <header class="head">
      <div class="title">{opts.title}</div>
      {#if opts.message}
        <div class="msg">{opts.message}</div>
      {/if}
    </header>

    <div class="body">
      <input
        bind:this={inputEl}
        bind:value
        class="ap-input field"
        placeholder={opts.placeholder ?? ""}
        onkeydown={onKeydown}
      />
    </div>

    <footer class="foot">
      <button type="button" class="ap-btn ghost" onclick={cancel}>
        Cancel
        <span class="ap-kbd">esc</span>
      </button>
      <div class="grow"></div>
      <button type="submit" class="ap-btn cta" disabled={!value.trim()}>
        {opts.confirmLabel ?? "Create"}
        <span class="ap-kbd">↵</span>
      </button>
    </footer>
  </form>
</dialog>

<style>
  /* Stretch the dialog over the whole viewport and centre the card inside —
     more deterministic than relying on the UA's default `margin: auto`
     behaviour which broke alignment on some WebKit builds. */
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
    padding: 18px 22px 10px;
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
    line-height: 1.45;
  }

  .body {
    padding: 6px 22px 14px;
  }
  .field {
    width: 100%;
    height: 34px;
    font-size: 13px;
    padding: 0 10px;
  }
  .field:focus {
    border-color: var(--accent-bd);
    box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.1);
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
</style>
