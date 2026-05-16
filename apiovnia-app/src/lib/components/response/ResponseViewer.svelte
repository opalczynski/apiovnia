<!--
  ResponseViewer — top-level container for the response panel. Owns the
  sub-tab state (raw / headers; pretty + preview land in Phase 4) and
  switches between the empty / error / live states.
-->
<script lang="ts">
  import ResponseHeader, {
    type ResponseTabId,
  } from "./ResponseHeader.svelte";
  import ResponseRaw from "./ResponseRaw.svelte";
  import ResponseHeaders from "./ResponseHeaders.svelte";
  import ResponsePretty from "./ResponsePretty.svelte";
  import ResponseSent from "./ResponseSent.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { formatDuration } from "./format";

  let activeTab = $state<ResponseTabId>("pretty");

  /**
   * Elapsed time while a request is in-flight. Ticks 5×/s — fast enough
   * to feel live, slow enough that the digits don't visibly flicker.
   */
  let elapsedMs = $state(0);

  $effect(() => {
    if (!app.executing) {
      elapsedMs = 0;
      return;
    }
    const startedAt = performance.now();
    elapsedMs = 0;
    const id = setInterval(() => {
      elapsedMs = Math.round(performance.now() - startedAt);
    }, 200);
    return () => clearInterval(id);
  });
</script>

<div class="resp">
  {#if app.executing}
    <div class="state">
      <span class="spinner"></span>
      <span>Sending request… <span class="elapsed mono">{formatDuration(elapsedMs)}</span></span>
    </div>
  {:else if app.executionError}
    <div class="state error">
      <strong>Request failed</strong>
      <span class="msg mono">{app.executionError}</span>
      <span class="hint">
        Check the URL, method, and your network. Errors and partial responses
        are logged to the request history.
      </span>
    </div>
  {:else if app.currentResponse}
    {@const r = app.currentResponse}
    <ResponseHeader
      response={r}
      {activeTab}
      onTabChange={(t) => (activeTab = t)}
    >
      {#if activeTab === "pretty"}
        <ResponsePretty response={r} />
      {:else if activeTab === "headers"}
        <ResponseHeaders response={r} />
      {:else if activeTab === "request"}
        <ResponseSent response={r} />
      {:else if activeTab === "raw"}
        <ResponseRaw response={r} />
      {/if}
    </ResponseHeader>
  {:else}
    <div class="state idle faint">
      No response yet — hit <span class="ap-kbd">⌘↵</span> or the Send button.
    </div>
  {/if}
</div>

<style>
  .resp {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--surface);
  }
  .state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    color: var(--fg-muted);
    font-size: 12.5px;
    text-align: center;
  }
  .state.idle {
    flex-direction: row;
    gap: 8px;
  }
  .faint {
    color: var(--fg-faint);
  }
  .state.error strong {
    color: var(--err);
    font-weight: 600;
    font-size: 13px;
  }
  .state.error .msg {
    color: var(--fg-dim);
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 11.5px;
    max-width: 560px;
    word-break: break-word;
  }
  .state.error .hint {
    font-size: 11.5px;
    color: var(--fg-faint);
    max-width: 460px;
    line-height: 1.5;
  }
  .elapsed {
    color: var(--fg-dim);
    font-size: 11.5px;
    margin-left: 4px;
    font-variant-numeric: tabular-nums;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
