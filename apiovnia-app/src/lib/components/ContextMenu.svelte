<!--
  ContextMenu — small popover positioned at a viewport point. Click outside
  or Esc closes. Anchored at `{ x, y }` (page coordinates); the parent owns
  open/close state and supplies the items.

  Items with `children` open a nested submenu on hover (anchored to the
  parent row's right edge). The submenu reuses this same component
  recursively — same look + close-on-outside-click semantics.
-->
<script module lang="ts">
  import type { IconName } from "$lib/components/icons";

  export type MenuItem = {
    label: string;
    icon?: IconName;
    danger?: boolean;
    /** Returning a Promise keeps the menu open until it settles. */
    onClick?: () => void | Promise<void>;
    /** Visual separator before this item. */
    separatorBefore?: boolean;
    /** When set, hover (or click) opens a nested submenu instead of firing
     *  `onClick`. The parent item itself becomes non-actionable. */
    children?: MenuItem[];
  };
</script>

<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  // Self-import for recursive submenu rendering (Svelte 5 way; the older
  // `<svelte:self>` is deprecated).
  import ContextMenu from "$lib/components/ContextMenu.svelte";

  type Props = {
    items: MenuItem[];
    x: number;
    y: number;
    onClose: () => void;
    /** True for nested submenus — suppresses the outside-click handler so
     *  they don't fight with the root menu's. The root listens once for
     *  the whole tree. */
    nested?: boolean;
  };

  const { items, x, y, onClose, nested = false }: Props = $props();

  let rootEl: HTMLDivElement | undefined = $state();
  /** Index of the row whose submenu is currently open (or null). */
  let openSubIndex = $state<number | null>(null);
  /** Anchor coords for the open submenu — computed from the row's rect. */
  let subAnchor = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  // Flip into the viewport if we'd overflow on the right or bottom.
  const style = $derived.by(() => {
    const w = 220;
    const h = Math.min(items.length * 28 + 8, 360);
    const maxX =
      (typeof window === "undefined" ? 1400 : window.innerWidth) - w - 8;
    const maxY =
      (typeof window === "undefined" ? 900 : window.innerHeight) - h - 8;
    return `left: ${Math.min(Math.max(8, x), maxX)}px; top: ${Math.min(Math.max(8, y), maxY)}px;`;
  });

  function onDocClick(e: MouseEvent) {
    if (nested) return; // root owns the outside-click handler
    if (!rootEl) return;
    // Walk up from the click target; if it lives inside *any* currently-
    // open `.menu` (root OR a recursive submenu rendered as a sibling),
    // keep the tree open. `querySelector` only returns the first match,
    // which would treat clicks on a submenu row as outside-click and
    // close everything on mousedown — before the row's `click` fires.
    const t = e.target as Node;
    const allMenus = document.querySelectorAll(".menu");
    for (const m of allMenus) {
      if (m.contains(t)) return;
    }
    onClose();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  $effect(() => {
    if (nested) return;
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onKeydown);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKeydown);
    };
  });

  async function trigger(item: MenuItem) {
    if (item.children) return; // submenu opens via hover; click is a no-op
    try {
      await item.onClick?.();
    } finally {
      onClose();
    }
  }

  function openSub(i: number, ev: MouseEvent | FocusEvent) {
    const item = items[i];
    if (!item?.children) {
      openSubIndex = null;
      return;
    }
    openSubIndex = i;
    const row = ev.currentTarget as HTMLElement;
    const r = row.getBoundingClientRect();
    // Anchor the submenu just past the row's right edge so it visually
    // hangs off the parent.
    subAnchor = { x: r.right - 4, y: r.top - 4 };
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
      class:has-sub={item.children != null}
      class:active={openSubIndex === i}
      onclick={() => void trigger(item)}
      onmouseenter={(e) => (item.children ? openSub(i, e) : (openSubIndex = null))}
      onfocus={(e) => (item.children ? openSub(i, e) : (openSubIndex = null))}
      role="menuitem"
    >
      {#if item.icon}
        <span class="icon"><Icon d={IC[item.icon]} /></span>
      {:else}
        <span class="icon"></span>
      {/if}
      <span class="label">{item.label}</span>
      {#if item.children}
        <span class="caret"><Icon d={IC.chevronR} size={11} /></span>
      {/if}
    </button>
  {/each}
</div>

{#if openSubIndex != null}
  {@const subItems = items[openSubIndex]?.children}
  {#if subItems}
    <ContextMenu
      items={subItems}
      x={subAnchor.x}
      y={subAnchor.y}
      onClose={() => {
        openSubIndex = null;
        onClose();
      }}
      nested
    />
  {/if}
{/if}

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
  .row:hover,
  .row.active {
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
  .row.has-sub {
    cursor: default;
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
  .caret {
    color: var(--fg-muted);
    display: inline-flex;
    align-items: center;
    margin-right: -2px;
  }
  .row:hover .caret,
  .row.active .caret {
    color: var(--fg);
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 4px 4px;
  }
</style>
