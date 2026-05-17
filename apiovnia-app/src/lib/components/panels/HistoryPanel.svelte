<!--
  HistoryPanel — slide-in overlay anchored to the left edge. Triggered by
  the History icon in ProjectsPanel's footer (the design's intent —
  bottom-left of the shell). Lists the last ~200 executions newest-first
  with a small status pill + timing line. Click → restore that response
  into the response viewer and navigate to the originating request.

  Backed by `commands::execution::list_history` (cap 200) +
  `get_history_response` (rehydrates one row's full `ExecutionResult`).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import Icon from "$lib/components/Icon.svelte";
  import MethodBadge from "$lib/components/MethodBadge.svelte";
  import { IC } from "$lib/components/icons";
  import { app } from "$lib/stores/app.svelte";
  import {
    formatBytes as _formatBytes,
    formatDuration,
    statusKind,
  } from "$lib/components/response/format";
  import type { HistoryRow, HttpMethod } from "$lib/types/domain";

  type Props = { onClose: () => void };
  let { onClose }: Props = $props();

  // Local filter (text match over request name / url / method / project /
  // collection — quick triage when 200 rows is a lot).
  let filterText = $state("");
  const q = $derived(filterText.trim().toLowerCase());
  const filtered = $derived.by(() => {
    const rows = app.historyEntries;
    if (!q) return rows;
    return rows.filter((r) => {
      const hay = [
        r.requestName,
        r.url,
        r.finalUrl,
        r.method,
        r.projectName,
        r.collectionName,
        r.environmentName,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return hay.includes(q);
    });
  });

  // Esc closes — capture at the panel level so it doesn't fight with other
  // modals.
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  // ---------------------------------------------------------------------------
  // Per-row helpers
  // ---------------------------------------------------------------------------

  /** Method as our typed enum (for MethodBadge). Defaults to GET when blank. */
  function asMethod(m: string | null): HttpMethod {
    const upper = (m ?? "GET").toUpperCase();
    switch (upper) {
      case "POST":
      case "PUT":
      case "PATCH":
      case "DELETE":
      case "HEAD":
      case "OPTIONS":
        return upper;
      default:
        return "GET";
    }
  }

  /** Pretty timestamp — same day → HH:MM; older → date + HH:MM. */
  function fmtTime(epochMillis: number): string {
    const d = new Date(epochMillis);
    const now = new Date();
    const sameDay =
      d.getFullYear() === now.getFullYear() &&
      d.getMonth() === now.getMonth() &&
      d.getDate() === now.getDate();
    const hh = String(d.getHours()).padStart(2, "0");
    const mm = String(d.getMinutes()).padStart(2, "0");
    if (sameDay) return `${hh}:${mm}`;
    const mon = d.toLocaleString(undefined, { month: "short" });
    return `${d.getDate()} ${mon} · ${hh}:${mm}`;
  }

  /** "200 · 1.20 s" line. Errors render as "Error · {message}". */
  function statusLabel(r: HistoryRow): string {
    if (r.errorMessage) return `Error · ${r.errorMessage}`;
    const bits: string[] = [];
    if (r.statusCode != null) bits.push(String(r.statusCode));
    if (r.durationMs != null) bits.push(formatDuration(r.durationMs));
    return bits.join(" · ");
  }

  function kindFor(r: HistoryRow): "ok" | "warn" | "err" {
    if (r.errorMessage) return "err";
    if (r.statusCode == null) return "warn";
    return statusKind(r.statusCode);
  }

  async function open(r: HistoryRow) {
    await app.openHistoryEntry(r);
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="false"
  aria-label="Request history"
  tabindex="-1"
>
  <header class="hdr">
    <div class="hdr-title">
      <span class="hdr-icon"><Icon d={IC.history} /></span>
      <h2>History</h2>
      <span class="count">{app.historyEntries.length}</span>
    </div>
    <div class="hdr-actions">
      <button
        class="ap-btn icon sm ghost"
        title="Refresh"
        onclick={() => void app.refreshHistory()}
      >
        <Icon d={IC.refresh} />
      </button>
      <button
        class="ap-btn icon sm ghost"
        title="Close"
        aria-label="Close history panel"
        onclick={onClose}
      >
        <Icon d={IC.x} />
      </button>
    </div>
  </header>

  <div class="filter-row">
    <div class="search-box">
      <span class="muted"><Icon d={IC.search} /></span>
      <input
        bind:value={filterText}
        class="ap-input bare"
        placeholder="Filter by name, url, method…"
        autocomplete="off"
        spellcheck="false"
      />
      {#if filterText}
        <button
          class="ap-btn icon sm ghost clear-btn"
          title="Clear filter"
          aria-label="Clear filter"
          onclick={() => (filterText = "")}
        >
          <Icon d={IC.x} size={11} />
        </button>
      {/if}
    </div>
  </div>

  <div class="body">
    {#if app.historyLoading && app.historyEntries.length === 0}
      <div class="empty">Loading history…</div>
    {:else if app.historyEntries.length === 0}
      <div class="empty">
        <Icon d={IC.history} />
        <span>
          No requests yet. Hit <span class="ap-kbd">⌘↵</span> on the active
          request to add one.
        </span>
      </div>
    {:else if filtered.length === 0}
      <div class="empty">No history matches "{filterText}"</div>
    {:else}
      <ul class="list">
        {#each filtered as r (r.id)}
          <li>
            <button class="row" onclick={() => open(r)} type="button">
              <span class="row-method">
                <MethodBadge method={asMethod(r.method)} />
              </span>
              <span class="row-main">
                <span class="row-name">
                  {r.requestName ?? "(deleted request)"}
                </span>
                <span class="row-sub">
                  {#if r.collectionName}
                    <span class="crumb">{r.collectionName}</span>
                    <span class="dot">·</span>
                  {/if}
                  <span class="url" title={r.finalUrl ?? r.url ?? ""}>
                    {r.url ?? r.finalUrl ?? ""}
                  </span>
                </span>
              </span>
              <span class="row-meta">
                <span class={`ap-status ${kindFor(r)}`}>{statusLabel(r)}</span>
                <span class="row-time" title={new Date(r.executedAt).toLocaleString()}>
                  {fmtTime(r.executedAt)}
                </span>
                {#if r.environmentName}
                  <span class="env">{r.environmentName}</span>
                {/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<!-- Click-outside scrim — closing happens on backdrop click, Esc, or × -->
<button
  type="button"
  class="scrim"
  aria-label="Close history panel"
  onclick={onClose}
></button>

<style>
  /* The panel slides in from the left. We keep it as a fixed overlay rather
     than re-flowing the layout — that keeps Send / response viewer state
     intact behind the panel. */
  .overlay {
    position: fixed;
    top: 32px;
    left: 0;
    bottom: 0;
    width: 460px;
    max-width: 90vw;
    background: var(--surface);
    border-right: 1px solid var(--border);
    box-shadow: 4px 0 24px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    z-index: 850;
    animation: slide-in 180ms ease-out;
  }
  @keyframes slide-in {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(0);
    }
  }
  .scrim {
    position: fixed;
    inset: 0;
    background: transparent;
    border: 0;
    cursor: default;
    z-index: 840;
  }

  .hdr {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 8px;
    border-bottom: 1px solid var(--border);
  }
  .hdr-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg);
  }
  .hdr-icon {
    color: var(--fg-muted);
    display: inline-flex;
  }
  .hdr-title h2 {
    margin: 0;
    font: 600 13px/1 var(--ui);
    letter-spacing: 0.02em;
  }
  .count {
    background: var(--surface-2);
    color: var(--fg-muted);
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 10.5px;
    font-weight: 600;
  }
  .hdr-actions {
    display: flex;
    gap: 4px;
  }

  .filter-row {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-soft);
  }
  .search-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    height: 26px;
    border-radius: 6px;
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .ap-input.bare {
    border: 0;
    background: transparent;
    padding: 0;
    flex: 1;
    height: 22px;
  }
  .muted {
    color: var(--fg-muted);
    display: inline-flex;
  }
  .clear-btn {
    width: 16px;
    height: 16px;
    padding: 0;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
  .empty {
    padding: 32px 16px;
    text-align: center;
    color: var(--fg-muted);
    font-size: 11.5px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .empty :global(.ap-kbd) {
    margin: 0 2px;
  }

  .list {
    list-style: none;
    padding: 4px 0;
    margin: 0;
  }
  .row {
    width: 100%;
    background: transparent;
    border: 0;
    padding: 8px 12px;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
    align-items: start;
    text-align: left;
    cursor: pointer;
    color: var(--fg);
    border-bottom: 1px solid var(--border-soft);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row-method {
    margin-top: 1px;
  }
  .row-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row-name {
    font: 500 12px/1.2 var(--ui);
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-sub {
    font: 11px/1.2 var(--mono);
    color: var(--fg-muted);
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .crumb {
    color: var(--fg-muted);
    font: 500 10.5px/1.2 var(--ui);
  }
  .dot {
    color: var(--fg-faint);
  }
  .url {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
  }
  .row-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    font-size: 11px;
    color: var(--fg-muted);
    white-space: nowrap;
  }
  .row-time {
    font: 11px/1 var(--mono);
    color: var(--fg-muted);
  }
  .env {
    font: 500 10px/1 var(--ui);
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
