<!--
  ResponseHeaders — read-only key/value list of response headers, preserves
  receive order. Lowercases names (standard wire convention).
-->
<script lang="ts">
  import type { ExecutionResult } from "$lib/types/domain";
  import HeadersGrid from "./HeadersGrid.svelte";

  type Props = { response: ExecutionResult };
  const { response }: Props = $props();
</script>

<div class="wrap">
  <HeadersGrid
    headers={response.headers}
    lowercaseNames
    emptyMessage="No response headers."
  />
</div>

<style>
  .wrap {
    flex: 1;
    overflow: auto;
    background: var(--bg);
  }
</style>
