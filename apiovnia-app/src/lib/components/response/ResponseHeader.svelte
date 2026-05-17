<!--
  ResponseHeader — strip at the top of the response panel with status pill,
  time, size, content-type + response sub-tabs (Raw / Headers; Pretty +
  Preview ship in Phase 4).
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import Tabs, { type TabSpec } from "$lib/components/ui/Tabs.svelte";
  import type { ExecutionResult } from "$lib/types/domain";
  import { formatBytes, formatDuration, statusKind } from "./format";

  export type ResponseTabId = "pretty" | "headers" | "request" | "raw";

  type Props = {
    response: ExecutionResult;
    activeTab: ResponseTabId;
    onTabChange: (id: ResponseTabId) => void;
    children: import("svelte").Snippet;
  };

  const { response, activeTab, onTabChange, children }: Props = $props();

  let tabs = $derived<TabSpec<ResponseTabId>[]>([
    { id: "pretty", label: "Pretty" },
    { id: "headers", label: "Headers", count: response.headers.length },
    { id: "request", label: "Request" },
    { id: "raw", label: "Raw" },
  ]);

  function copyBody() {
    void navigator.clipboard.writeText(response.body);
  }
</script>

<div class="bar">
  <span class="ap-status {statusKind(response.status)}">
    {response.status} {response.statusText}
  </span>

  <div class="meta mono">
    <span class="metric">
      <span class="faint">time</span>
      <span class="dim">{formatDuration(response.durationMs)}</span>
    </span>
    <span class="metric">
      <span class="faint">size</span>
      <span class="dim">{formatBytes(response.sizeBytes)}</span>
    </span>
    {#if response.contentType}
      <span class="faint">{response.contentType}</span>
    {/if}
    {#if response.bodyTruncated}
      <span class="warn">truncated</span>
    {/if}
  </div>

  <span class="grow"></span>

  <button
    class="ap-btn sm ghost copy-btn"
    title="Copy response body"
    onclick={copyBody}
  >
    <Icon d={IC.copy} />
    <span>Copy response body</span>
  </button>
</div>

<Tabs value={activeTab} {tabs} onChange={onTabChange}>
  {@render children()}
</Tabs>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .meta {
    display: flex;
    gap: 14px;
    color: var(--fg-muted);
    font-size: 11px;
  }
  /* Inline-flex so the "time" label and "1.70 s" value keep a tab-width
     gap regardless of whitespace collapsing in the rendered HTML. */
  .metric {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
  }
  .dim {
    color: var(--fg-dim);
  }
  /* Local "faint" tier — labels (`time`, `size`) and the content-type
     string sit on --surface in mono at 11 px. Even fg-muted reads a bit
     thin there, so we pin them to fg-muted explicitly (not fg-faint —
     these are scanning targets, not decorations). */
  .faint {
    color: var(--fg-muted);
  }
  .warn {
    color: var(--warn);
    font-weight: 600;
  }
  .grow {
    flex: 1;
  }
  .copy-btn {
    gap: 6px;
  }
</style>
