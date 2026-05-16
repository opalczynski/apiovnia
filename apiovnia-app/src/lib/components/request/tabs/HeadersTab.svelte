<script lang="ts">
  import KeyValueTable from "../KeyValueTable.svelte";
  import type { KeyValue, Request } from "$lib/types/domain";

  type Props = {
    request: Request;
    onPatch: (patch: Partial<Request>) => void;
  };

  const { request, onPatch }: Props = $props();

  function setHeaders(next: KeyValue[]) {
    onPatch({ headers: next });
  }
</script>

<div class="wrap">
  <KeyValueTable
    rows={request.headers}
    onChange={setHeaders}
    keyPlaceholder="Header"
    valuePlaceholder="Value"
  />
</div>

<style>
  .wrap {
    flex: 1;
    overflow: auto;
    background: var(--bg);
  }
</style>
