<script lang="ts">
  import Resizer from "./Resizer.svelte";
  import { panels } from "$lib/stores/panels.svelte";
  import type { Snippet } from "svelte";

  type Props = {
    left: Snippet;
    middle: Snippet;
    right: Snippet;
  };

  const { left, middle, right }: Props = $props();
</script>

<div class="layout">
  <aside class="pane left" style="width: {panels.leftWidth}px">
    {@render left()}
  </aside>

  <Resizer onDrag={(d) => (panels.leftWidth = panels.leftWidth + d)} />

  <aside class="pane middle" style="width: {panels.middleWidth}px">
    {@render middle()}
  </aside>

  <Resizer onDrag={(d) => (panels.middleWidth = panels.middleWidth + d)} />

  <section class="pane right">
    {@render right()}
  </section>
</div>

<style>
  .layout {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .pane.left {
    background: var(--bg);
  }
  .pane.middle {
    background: var(--surface);
    border-left: 1px solid var(--border);
  }
  .pane.right {
    flex: 1;
    background: var(--bg);
    border-left: 1px solid var(--border);
  }
</style>
