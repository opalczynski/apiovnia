<!--
  HeadersGrid — shared two-column table used by both Response Headers tab
  and the Headers section of the Request tab. Single styling so the two
  reads identically.
-->
<script lang="ts">
  import type { HeaderEntry } from "$lib/types/domain";

  type Props = {
    headers: HeaderEntry[];
    /** Force lowercase the header names (response convention). */
    lowercaseNames?: boolean;
    emptyMessage?: string;
  };

  const { headers, lowercaseNames = false, emptyMessage = "No headers." }: Props = $props();
</script>

{#if headers.length === 0}
  <div class="empty">{emptyMessage}</div>
{:else}
  <div class="grid">
    {#each headers as h, i (i)}
      <div class="cell name mono" class:lower={lowercaseNames}>{h.name}</div>
      <div class="cell value mono">{h.value}</div>
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) 2.5fr;
    align-items: stretch;
  }
  .cell {
    padding: 6px 12px;
    font-size: 11.5px;
    line-height: 16px;
    border-bottom: 1px solid var(--border-soft);
    overflow-wrap: anywhere;
  }
  .cell.name {
    color: var(--fg-dim);
    background: var(--surface);
  }
  .cell.name.lower {
    text-transform: lowercase;
  }
  .cell.value {
    color: var(--fg);
  }
  .empty {
    padding: 14px 12px;
    color: var(--fg-faint);
    font-size: 11.5px;
    font-style: italic;
  }
</style>
