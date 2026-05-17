<script lang="ts">
  import KeyValueTable from "../KeyValueTable.svelte";
  import type { KeyValue, Request } from "$lib/types/domain";

  type Props = {
    request: Request;
    onPatch: (patch: Partial<Request>) => void;
  };

  const { request, onPatch }: Props = $props();

  function setParams(next: KeyValue[]) {
    onPatch({ params: next });
  }
</script>

<div class="wrap">
  <KeyValueTable
    rows={request.params}
    onChange={setParams}
    keyPlaceholder="Param"
    valuePlaceholder="Value"
  />
</div>

<style>
  .wrap {
    flex: 1;
    overflow: auto;
    background: var(--bg);
    /* Breathing room so the first table row doesn't hug the tab divider. */
    padding-top: 8px;
  }
</style>
