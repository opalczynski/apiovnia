<!--
  Popover — generic floating panel anchored to an HTMLElement (or fixed point).
  Positioning is computed once on open + on window resize. Click-outside and
  Esc close. Parent controls `open` via $bindable.

  Placements: bottom-start (default), bottom-end, top-start, top-end.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type Placement =
    | "bottom-start"
    | "bottom-end"
    | "top-start"
    | "top-end";

  type Props = {
    /** HTML element to anchor under/over. */
    anchor: HTMLElement | null | undefined;
    /** Two-way open state. */
    open: boolean;
    placement?: Placement;
    /** Min-width = anchor width (e.g. for Select pickers). */
    matchAnchorWidth?: boolean;
    /** Children snippet. */
    children: Snippet;
    onOpenChange?: (next: boolean) => void;
  };

  let {
    anchor,
    open = $bindable(),
    placement = "bottom-start",
    matchAnchorWidth = false,
    children,
    onOpenChange,
  }: Props = $props();

  let rootEl: HTMLDivElement | undefined = $state();
  let style = $state("");

  function reposition() {
    if (!anchor || !rootEl) return;
    const r = anchor.getBoundingClientRect();
    const popH = rootEl.offsetHeight || 0;
    const popW = rootEl.offsetWidth || 0;
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    let top: number;
    let left: number;

    switch (placement) {
      case "bottom-end":
        top = r.bottom + 4;
        left = r.right - popW;
        break;
      case "top-start":
        top = r.top - popH - 4;
        left = r.left;
        break;
      case "top-end":
        top = r.top - popH - 4;
        left = r.right - popW;
        break;
      case "bottom-start":
      default:
        top = r.bottom + 4;
        left = r.left;
        break;
    }

    // Clamp inside viewport with 8px margin.
    left = Math.max(8, Math.min(left, vw - popW - 8));
    top = Math.max(8, Math.min(top, vh - popH - 8));

    const widthStyle = matchAnchorWidth ? `min-width: ${r.width}px;` : "";
    style = `position: fixed; top: ${top}px; left: ${left}px; z-index: 90; ${widthStyle}`;
  }

  function close() {
    if (!open) return;
    open = false;
    onOpenChange?.(false);
  }

  function onDocPointerDown(e: PointerEvent) {
    if (!rootEl) return;
    const target = e.target as Node;
    if (rootEl.contains(target)) return;
    if (anchor && anchor.contains(target)) return;
    close();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }
  function onResize() {
    reposition();
  }

  $effect(() => {
    if (!open) return;
    // First paint to measure, then position, then listeners.
    queueMicrotask(reposition);
    window.addEventListener("resize", onResize);
    window.addEventListener("scroll", onResize, true);
    document.addEventListener("pointerdown", onDocPointerDown);
    document.addEventListener("keydown", onKeydown);
    return () => {
      window.removeEventListener("resize", onResize);
      window.removeEventListener("scroll", onResize, true);
      document.removeEventListener("pointerdown", onDocPointerDown);
      document.removeEventListener("keydown", onKeydown);
    };
  });
</script>

{#if open}
  <div bind:this={rootEl} class="popover" {style} role="dialog">
    {@render children()}
  </div>
{/if}

<style>
  .popover {
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(0, 0, 0, 0.3);
    color: var(--fg);
    overflow: hidden;
  }
</style>
