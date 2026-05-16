<!--
  ContextMenu — small popover positioned at a viewport point. Click outside
  or Esc closes. Anchored at `{ x, y }` (page coordinates); the parent owns
  open/close state and supplies the items.
-->
<script module lang="ts">
  import type { IconName } from "$lib/components/icons";

  export type MenuItem = {
    label: string;
    icon?: IconName;
    danger?: boolean;
    /** Returning a Promise keeps the menu open until it settles. */
    onClick: () => void | Promise<void>;
    /** Visual separator before this item. */
    separatorBefore?: boolean;
  };
</script>

<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";

  type Props = {
    items: MenuItem[];
    x: number;
    y: number;
    onClose: () => void;
  };

  const { items, x, y, onClose }: Props = $props();

  let rootEl: HTMLDivElement | undefined = $state();

  // Flip into the viewport if we'd overflow on the right or bottom.
  let style = $derived.by(() => {
    const w = 220;
    const h = Math.min(items.length * 28 + 8, 360);
    const maxX = (typeof window === "undefined" ? 1400 : window.innerWidth) - w - 8;
    const maxY = (typeof window === "undefined" ? 900 : window.innerHeight) - h - 8;
    return `left: ${Math.min(Math.max(8, x), maxX)}px; top: ${Math.min(Math.max(8, y), maxY)}px;`;
  });

  function onDocClick(e: MouseEvent) {
    if (!rootEl) return;
    if (!rootEl.contains(e.target as Node)) onClose();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  $effect(() => {
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onKeydown);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKeydown);
    };
  });

  async function trigger(item: MenuItem) {
    try {
      await item.onClick();
    } finally {
      onClose();
    }
  }
</script>

<div bind:this={rootEl} class="menu" {style} role="menu">
  {#each items as item, i (i)}
    {#if item.separatorBefore}
      <div class="sep" role="separator"></div>
    {/if}
    <button
      type="button"
      class="row"
      class:danger={item.danger}
      onclick={() => void trigger(item)}
      role="menuitem"
    >
      {#if item.icon}
        <span class="icon"><Icon d={IC[item.icon]} /></span>
      {:else}
        <span class="icon"></span>
      {/if}
      <span class="label">{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 100;
    min-width: 180px;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(0, 0, 0, 0.3);
    font: 12px/1 var(--ui);
  }
  .row {
    width: 100%;
    height: 26px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    background: transparent;
    border: 0;
    border-radius: 5px;
    color: var(--fg-dim);
    cursor: pointer;
    text-align: left;
  }
  .row:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .row.danger {
    color: var(--err);
  }
  .row.danger:hover {
    background: color-mix(in srgb, var(--err) 12%, transparent);
    color: var(--err);
  }
  .icon {
    width: 14px;
    height: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .row.danger .icon {
    color: var(--err);
  }
  .label {
    flex: 1;
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 4px 4px;
  }
</style>
