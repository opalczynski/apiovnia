<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import MethodBadge from "$lib/components/MethodBadge.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/ContextMenu.svelte";
  import Popover from "$lib/components/ui/Popover.svelte";
  import { app, SNIPPET_FORMATS, snippetFormatLabel } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type { HttpMethod, RequestSummary } from "$lib/types/domain";

  // ---------------------------------------------------------------------------
  // Filters — text (name + URL) AND method multi-select. Both are visual-only;
  // selection persists even if the active request is hidden by the filter.
  // ---------------------------------------------------------------------------

  const METHODS: HttpMethod[] = [
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "HEAD",
    "OPTIONS",
  ];

  let filterText = $state("");
  let methodFilter = $state<Set<HttpMethod>>(new Set());
  let methodMenuOpen = $state(false);
  let methodBtnEl: HTMLButtonElement | undefined = $state();

  const q = $derived(filterText.trim().toLowerCase());

  const filtered = $derived.by(() =>
    app.requests.filter((r) => {
      if (methodFilter.size > 0 && !methodFilter.has(r.method as HttpMethod)) return false;
      if (q) {
        const hay = `${r.name} ${r.url ?? ""}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    }),
  );

  const filterActive = $derived(q.length > 0 || methodFilter.size > 0);

  function toggleMethod(m: HttpMethod) {
    const next = new Set(methodFilter);
    if (next.has(m)) next.delete(m);
    else next.add(m);
    methodFilter = next;
  }

  function clearMethodFilter() {
    methodFilter = new Set();
  }

  function clearAll() {
    filterText = "";
    clearMethodFilter();
  }

  async function newRequest() {
    if (!app.activeCollectionId) return;
    const name = await dialogs.prompt({
      title: "New request",
      message: `In collection: ${app.activeCollection?.name ?? ""}`,
      placeholder: "e.g. Login, List users…",
      defaultValue: "Untitled request",
      confirmLabel: "Create request",
    });
    if (name) await app.createRequest(name);
  }

  async function renameRequest(r: RequestSummary) {
    const next = await dialogs.prompt({
      title: "Rename request",
      placeholder: "Request name",
      defaultValue: r.name,
      confirmLabel: "Rename",
    });
    if (next && next !== r.name) await app.renameRequest(r.id, next);
  }

  async function deleteRequest(r: RequestSummary) {
    const ok = await dialogs.confirm({
      title: `Delete "${r.name}"?`,
      message: "This request will be removed. Cannot be undone.",
      confirmLabel: "Delete request",
      danger: true,
    });
    if (ok) await app.deleteRequest(r.id);
  }

  type MenuPos = { x: number; y: number; items: MenuItem[] };
  let menu = $state<MenuPos | null>(null);

  function requestMenu(r: RequestSummary): MenuItem[] {
    return [
      { label: "Rename", icon: "pencil", onClick: () => renameRequest(r) },
      {
        label: "Copy as…",
        icon: "copy",
        children: SNIPPET_FORMATS.map((f) => ({
          label: snippetFormatLabel(f),
          onClick: () => app.copyRequestAsSnippet(r.id, f),
        })),
      },
      {
        label: "Delete request",
        icon: "trash",
        danger: true,
        separatorBefore: true,
        onClick: () => deleteRequest(r),
      },
    ];
  }

  function openMenu(e: MouseEvent, items: MenuItem[]) {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "contextmenu") {
      menu = { x: e.clientX, y: e.clientY, items };
    } else {
      const btn = e.currentTarget as HTMLElement;
      const r = btn.getBoundingClientRect();
      menu = { x: r.right - 4, y: r.bottom + 4, items };
    }
  }
</script>

<div class="panel">
  <div class="search-row">
    <div class="search-box">
      <span class="muted"><Icon d={IC.search} /></span>
      <input
        class="ap-input bare"
        placeholder="Filter requests…"
        bind:value={filterText}
        data-focus-target="mid"
        onkeydown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            filterText = "";
          }
        }}
      />
      {#if filterText}
        <button
          class="ap-btn icon sm ghost clear-btn"
          title="Clear text filter"
          onclick={() => (filterText = "")}
        >
          <Icon d={IC.x} size={11} />
        </button>
      {/if}
    </div>
    <button
      bind:this={methodBtnEl}
      class="ap-btn icon sm ghost"
      class:filter-on={methodFilter.size > 0}
      title={methodFilter.size > 0
        ? `Method filter: ${[...methodFilter].join(", ")}`
        : "Filter by method"}
      onclick={() => (methodMenuOpen = !methodMenuOpen)}
    >
      <Icon d={IC.filter} />
      {#if methodFilter.size > 0}
        <span class="filter-badge mono">{methodFilter.size}</span>
      {/if}
    </button>
    <button
      class="ap-btn icon sm ghost"
      title="New request"
      onclick={newRequest}
      disabled={!app.activeCollectionId}
    >
      <Icon d={IC.plus} />
    </button>
  </div>

  <Popover anchor={methodBtnEl} bind:open={methodMenuOpen} placement="bottom-end">
    <div class="method-menu">
      <div class="method-menu-head">
        <span>Filter by method</span>
        {#if methodFilter.size > 0}
          <button class="ap-btn sm ghost" onclick={clearMethodFilter}>Clear</button>
        {/if}
      </div>
      {#each METHODS as m (m)}
        {@const checked = methodFilter.has(m)}
        <button type="button" class="method-row" class:checked onclick={() => toggleMethod(m)}>
          <span class="check">
            {#if checked}<Icon d={IC.check} size={11} />{/if}
          </span>
          <MethodBadge method={m} />
        </button>
      {/each}
    </div>
  </Popover>

  {#if app.activeCollection}
    <div class="section-label">
      <span>{app.activeCollection.name}</span>
      <span class="dim">
        {#if filterActive}
          · {filtered.length} of {app.requests.length}
        {:else}
          · {app.requests.length} request{app.requests.length === 1 ? "" : "s"}
        {/if}
      </span>
      {#if filterActive}
        <button class="ap-btn sm ghost clear-link" onclick={clearAll}>Clear filter</button>
      {/if}
    </div>
  {/if}

  <div class="list">
    {#if !app.activeCollectionId}
      <div class="hint">Select a collection on the left.</div>
    {:else if app.requests.length === 0}
      <button type="button" class="empty" onclick={newRequest}>
        <Icon d={IC.plus} />
        <span>Create your first request</span>
      </button>
    {:else if filtered.length === 0}
      <div class="hint">No requests match the current filter.</div>
    {/if}

    {#each filtered as r (r.id)}
      {@const active = app.activeRequestId === r.id}
      <div
        class="ap-row big row"
        class:active
        onclick={() => void app.selectRequest(r.id)}
        oncontextmenu={(e) => openMenu(e, requestMenu(r))}
        role="button"
        tabindex="0"
        onkeydown={(e) => {
          if (e.key === "Enter") void app.selectRequest(r.id);
        }}
      >
        <MethodBadge method={r.method} />
        <div class="meta">
          <div class="title">{r.name}</div>
          <div class="mono subtitle" class:placeholder={!r.url}>
            {r.url || "add URL in editor"}
          </div>
        </div>
        <button
          type="button"
          class="more"
          title="Request menu"
          onclick={(e) => openMenu(e, requestMenu(r))}
          aria-label="Request actions"
        >
          <Icon d={IC.more} />
        </button>
      </div>
    {/each}
  </div>
</div>

{#if menu}
  <ContextMenu
    items={menu.items}
    x={menu.x}
    y={menu.y}
    onClose={() => (menu = null)}
  />
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface);
    min-width: 0;
  }
  .search-row {
    padding: 10px 8px 8px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .search-box {
    flex: 1;
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
  }

  .section-label {
    padding: 2px 10px 6px;
    font-size: 10.5px;
    color: var(--fg-muted);
    display: flex;
    align-items: center;
    gap: 6px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 600;
  }
  .dim {
    color: var(--fg-muted);
  }

  .list {
    padding: 0 6px;
    overflow: auto;
    flex: 1;
  }
  .hint {
    padding: 16px 10px;
    color: var(--fg-muted);
    font-size: 11.5px;
    font-style: italic;
  }

  .row {
    cursor: default;
  }
  .ap-row.big {
    height: 36px;
    align-items: center;
    padding: 0 8px;
    gap: 8px;
  }
  .meta {
    flex: 1;
    min-width: 0;
    line-height: 1.15;
    text-align: left;
  }
  .title {
    font-size: 12.5px;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subtitle {
    font-size: 10.5px;
    color: var(--fg-muted);
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subtitle.placeholder {
    font-style: italic;
    color: var(--fg-faint);
    opacity: 0.85;
  }

  .more {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 0;
    background: transparent;
    color: var(--fg-muted);
    border-radius: 4px;
    opacity: 0;
    transition:
      opacity 0.12s,
      color 0.12s,
      background 0.12s;
    cursor: pointer;
  }
  .row:hover .more,
  .row.active .more {
    opacity: 1;
  }
  .more:hover {
    background: var(--bg);
    color: var(--fg);
  }

  .clear-btn {
    width: 16px;
    height: 16px;
    padding: 0;
  }
  .clear-link {
    margin-left: auto;
    font-size: 10px;
    text-transform: none;
    letter-spacing: 0;
    height: 18px;
    padding: 0 6px;
  }
  .filter-on {
    color: var(--accent);
    background: var(--accent-bg);
  }
  .filter-on:hover {
    color: var(--accent-hi);
  }
  .filter-badge {
    position: absolute;
    top: -3px;
    right: -3px;
    background: var(--accent);
    color: #1a1102;
    font-size: 9px;
    font-weight: 700;
    line-height: 1;
    min-width: 12px;
    height: 12px;
    border-radius: 6px;
    padding: 0 3px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  /* The filter button needs `position: relative` for the badge */
  :global(.search-row .ap-btn.filter-on),
  :global(.search-row .ap-btn.icon) {
    position: relative;
  }

  /* Popover content */
  .method-menu {
    min-width: 180px;
    padding: 4px;
  }
  .method-menu-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
    font-weight: 600;
  }
  .method-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    background: transparent;
    border: 0;
    border-radius: 5px;
    color: var(--fg-dim);
    cursor: pointer;
    text-align: left;
  }
  .method-row:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .method-row .check {
    width: 14px;
    height: 14px;
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }
  .method-row.checked .check {
    background: var(--accent);
    border-color: var(--accent);
    color: #1a1102;
  }

  .empty {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    margin: 4px 0;
    background: transparent;
    border: 1px dashed var(--border-strong);
    border-radius: 6px;
    color: var(--fg-muted);
    font-size: 11.5px;
    cursor: pointer;
  }
  .empty:hover {
    color: var(--fg-dim);
    border-color: var(--accent-bd);
  }
</style>
