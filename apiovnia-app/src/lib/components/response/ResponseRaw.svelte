<!--
  ResponseRaw — body as a `<pre>`. For binary payloads we show a short note
  with the size and content-type instead of dumping base64.
-->
<script lang="ts">
  import type { ExecutionResult } from "$lib/types/domain";

  type Props = { response: ExecutionResult };
  const { response }: Props = $props();
</script>

<div class="wrap">
  {#if response.bodyKind === "empty"}
    <div class="empty">Empty response body.</div>
  {:else if response.bodyKind === "binarybase64"}
    <div class="empty">
      Binary payload ({response.contentType ?? "unknown type"}, base64-encoded
      {response.body.length.toLocaleString()} characters).
      <br />
      Phase 4 will add a hex / preview viewer; for now use the Copy button to
      grab the base64 string.
    </div>
  {:else}
    <pre class="body mono">{response.body}</pre>
  {/if}
</div>

<style>
  .wrap {
    flex: 1;
    overflow: auto;
    background: var(--bg);
  }
  .body {
    margin: 0;
    padding: 12px 14px;
    font-size: 12.5px;
    line-height: 20px;
    color: var(--fg);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .empty {
    padding: 24px;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.5;
    max-width: 520px;
  }
</style>
