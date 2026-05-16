<!--
  Resizer — pointer-driven splitter for left/middle panel widths and the
  request/response horizontal split.

  Two orientations: vertical (col-resize, default) and horizontal (row-resize).
  Emits the *delta* in pixels via onDrag; the parent decides what to update.
  Pointer capture keeps the drag working even when the cursor leaves the
  resizer's bounding box.
-->
<script lang="ts">
  type Props = {
    orientation?: "vertical" | "horizontal";
    onDrag: (deltaPx: number) => void;
  };

  const { orientation = "vertical", onDrag }: Props = $props();

  let dragging = $state(false);
  let startCoord = 0;
  let lastDelta = 0;

  function handlePointerDown(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    dragging = true;
    startCoord = orientation === "vertical" ? e.clientX : e.clientY;
    lastDelta = 0;
    e.preventDefault();
  }

  function handlePointerMove(e: PointerEvent) {
    if (!dragging) return;
    const current = orientation === "vertical" ? e.clientX : e.clientY;
    const delta = current - startCoord - lastDelta;
    lastDelta = current - startCoord;
    onDrag(delta);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!dragging) return;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    dragging = false;
  }
</script>

<div
  class={orientation === "vertical" ? "ap-resizer" : "ap-resizer-h"}
  class:dragging
  role="separator"
  aria-orientation={orientation}
  tabindex="-1"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
></div>
