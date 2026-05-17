<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";

  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import type { MenuItem } from "$lib/components/ContextMenu.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type {
    Collection,
    CollectionId,
    Project,
    ProjectId,
  } from "$lib/types/domain";

  // ---------------------------------------------------------------------------
  // Filter — case-insensitive substring match over project + collection names.
  // Selection is preserved when filter hides the active row (visual-only).
  // ⌘P / Ctrl+P focuses the input (per design hint).
  // ---------------------------------------------------------------------------

  let filterText = $state("");
  let filterEl: HTMLInputElement | undefined = $state();

  const q = $derived(filterText.trim().toLowerCase());

  const filteredProjects = $derived.by(() => {
    if (!q) return app.projects;
    return app.projects.filter((p) => p.name.toLowerCase().includes(q));
  });

  const filteredCollections = $derived.by(() => {
    if (!q) return app.collections;
    return app.collections.filter((c) => c.name.toLowerCase().includes(q));
  });

  onMount(() => {
    function handler(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "p") {
        e.preventDefault();
        filterEl?.focus();
        filterEl?.select();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  // ---------------------------------------------------------------------------
  // Create flows
  // ---------------------------------------------------------------------------

  async function newProject() {
    const name = await dialogs.prompt({
      title: "New project",
      message: "Projects are top-level groupings — pick a workspace name.",
      placeholder: "e.g. UDL, Resi4Rent…",
      confirmLabel: "Create project",
    });
    if (name) await app.createProject(name);
  }

  async function newCollection() {
    if (!app.activeProjectId) return;
    const name = await dialogs.prompt({
      title: "New collection",
      message: `In project: ${app.activeProject?.name ?? ""}`,
      placeholder: "e.g. Auth, Properties…",
      confirmLabel: "Create collection",
    });
    if (name) await app.createCollection(name);
  }

  // ---------------------------------------------------------------------------
  // Per-row actions
  // ---------------------------------------------------------------------------

  async function renameProject(p: Project) {
    const next = await dialogs.prompt({
      title: "Rename project",
      placeholder: "Project name",
      defaultValue: p.name,
      confirmLabel: "Rename",
    });
    if (next && next !== p.name) await app.renameProject(p.id, next);
  }

  async function deleteProject(p: Project) {
    const ok = await dialogs.confirm({
      title: `Delete "${p.name}"?`,
      message:
        "This permanently removes the project, all of its collections, and all of their requests. Cannot be undone.",
      confirmLabel: "Delete project",
      danger: true,
    });
    if (ok) await app.deleteProject(p.id);
  }

  async function renameCollection(c: Collection) {
    const next = await dialogs.prompt({
      title: "Rename collection",
      placeholder: "Collection name",
      defaultValue: c.name,
      confirmLabel: "Rename",
    });
    if (next && next !== c.name) await app.renameCollection(c.id, next);
  }

  async function deleteCollection(c: Collection) {
    const ok = await dialogs.confirm({
      title: `Delete "${c.name}"?`,
      message: "All requests in this collection will be removed. Cannot be undone.",
      confirmLabel: "Delete collection",
      danger: true,
    });
    if (ok) await app.deleteCollection(c.id);
  }

  // ---------------------------------------------------------------------------
  // Context menu plumbing — one shared menu per panel.
  // ---------------------------------------------------------------------------

  type MenuPos = { x: number; y: number; items: MenuItem[] };
  let menu = $state<MenuPos | null>(null);

  function projectMenu(p: Project): MenuItem[] {
    return [
      { label: "Rename", icon: "pencil", onClick: () => renameProject(p) },
      {
        label: "Import OpenAPI…",
        icon: "arrowR",
        separatorBefore: true,
        onClick: () => importOpenapiInto(p),
      },
      {
        label: "Delete project",
        icon: "trash",
        danger: true,
        separatorBefore: true,
        onClick: () => deleteProject(p),
      },
    ];
  }
  function collectionMenu(c: Collection): MenuItem[] {
    return [
      { label: "Rename", icon: "pencil", onClick: () => renameCollection(c) },
      {
        label: "Export OpenAPI…",
        icon: "send",
        separatorBefore: true,
        onClick: () => exportCollectionOpenapi(c),
      },
      {
        label: "Delete collection",
        icon: "trash",
        danger: true,
        separatorBefore: true,
        onClick: () => deleteCollection(c),
      },
    ];
  }

  // OpenAPI import / export flows. File picker via tauri-plugin-dialog;
  // store actions handle the IPC + OpLog rendering.

  async function importOpenapiInto(p: Project) {
    const path = await open({
      title: `Import OpenAPI into "${p.name}"`,
      multiple: false,
      directory: false,
      filters: [{ name: "OpenAPI", extensions: ["yaml", "yml", "json"] }],
    });
    if (typeof path === "string") {
      await app.importOpenapiForProject(p.id, path);
    }
  }

  async function exportCollectionOpenapi(c: Collection) {
    // The store owns the build → dialog → save → log dance so the suggested
    // filename can include the project name (which the IPC knows but the
    // panel doesn't always have at hand).
    await app.exportCollectionInteractive(c.id);
  }

  /**
   * Open menu either at cursor (right-click) or anchored under the `⋯` button.
   */
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

  // ---------------------------------------------------------------------------
  // Selection — click row body (not the ⋯) selects.
  // ---------------------------------------------------------------------------

  function selectProject(id: ProjectId) {
    void app.selectProject(id);
  }
  function selectCollection(id: CollectionId) {
    void app.selectCollection(id);
  }
</script>

<div class="panel">
  <div class="search-row">
    <div class="search-box">
      <span class="muted"><Icon d={IC.search} /></span>
      <input
        bind:this={filterEl}
        bind:value={filterText}
        class="ap-input bare"
        placeholder="Filter projects & collections…"
        onkeydown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            filterText = "";
            filterEl?.blur();
          }
        }}
      />
      {#if filterText}
        <button
          class="ap-btn icon sm ghost clear-btn"
          title="Clear filter"
          onclick={() => {
            filterText = "";
            filterEl?.focus();
          }}
        >
          <Icon d={IC.x} size={11} />
        </button>
      {:else}
        <span class="ap-kbd">⌘P</span>
      {/if}
    </div>
  </div>

  <section class="list">
    <div class="ap-sec collections-header">
      <span>Projects</span>
      <button
        class="ap-btn icon sm ghost"
        title="New project"
        onclick={newProject}
      >
        <Icon d={IC.plus} />
      </button>
    </div>
    {#if app.projects.length === 0}
      <button type="button" class="empty" onclick={newProject}>
        <Icon d={IC.plus} />
        <span>Create your first project</span>
      </button>
    {:else if filteredProjects.length === 0}
      <div class="no-match">No projects match "{filterText}"</div>
    {/if}
    {#each filteredProjects as p (p.id)}
      {@const active = app.activeProjectId === p.id}
      <div
        class="ap-row row"
        class:active
        onclick={() => selectProject(p.id)}
        oncontextmenu={(e) => openMenu(e, projectMenu(p))}
        role="button"
        tabindex="0"
        onkeydown={(e) => {
          if (e.key === "Enter") selectProject(p.id);
        }}
      >
        <span class="row-icon" class:dim={!active}>
          <Icon d={IC.folder} />
        </span>
        <span class="row-label">{p.name}</span>
        <button
          type="button"
          class="more"
          title="Project menu"
          onclick={(e) => openMenu(e, projectMenu(p))}
          aria-label="Project actions"
        >
          <Icon d={IC.more} />
        </button>
      </div>
    {/each}
  </section>

  {#if app.activeProject}
    <section class="list collections">
      <div class="ap-sec collections-header stacked">
        <div class="header-text">
          <div class="header-label">Collections</div>
          <div class="header-sub" title={app.activeProject.name}>
            {app.activeProject.name}
          </div>
        </div>
        <button
          class="ap-btn icon sm ghost"
          title="New collection"
          onclick={newCollection}
        >
          <Icon d={IC.plus} />
        </button>
      </div>

      {#if app.collections.length === 0}
        <button type="button" class="empty" onclick={newCollection}>
          <Icon d={IC.plus} />
          <span>New collection</span>
        </button>
      {:else if filteredCollections.length === 0}
        <div class="no-match">No collections match "{filterText}"</div>
      {/if}

      {#each filteredCollections as c (c.id)}
        {@const active = app.activeCollectionId === c.id}
        <div
          class="ap-row row"
          class:active
          onclick={() => selectCollection(c.id)}
          oncontextmenu={(e) => openMenu(e, collectionMenu(c))}
          role="button"
          tabindex="0"
          onkeydown={(e) => {
            if (e.key === "Enter") selectCollection(c.id);
          }}
        >
          <span class="row-icon dim"><Icon d={IC.collection} /></span>
          <span class="row-label">{c.name}</span>
          <button
            type="button"
            class="more"
            title="Collection menu"
            onclick={(e) => openMenu(e, collectionMenu(c))}
            aria-label="Collection actions"
          >
            <Icon d={IC.more} />
          </button>
        </div>
      {/each}
    </section>
  {/if}

  <div class="spacer"></div>

  <footer class="footer">
    <div class="avatar">A</div>
    <div class="who">
      <div class="who-name">Apiovnia</div>
      <div class="who-sub">Local · SQLite</div>
    </div>
    <button class="ap-btn icon sm ghost" title="History">
      <Icon d={IC.history} />
    </button>
  </footer>
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
    background: var(--bg);
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
    background: var(--surface);
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

  .list {
    padding: 0 6px;
  }
  .list.collections {
    padding: 8px 6px 0;
    border-top: 1px solid var(--border-soft);
    margin-top: 8px;
  }
  .collections-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  /* Stacked variant — "Collections" tiny label on top, full project name
     beneath. Keeps the `+` button vertically aligned with the header label,
     never gets squeezed by a long project name. */
  .collections-header.stacked {
    align-items: flex-start;
    padding-bottom: 6px;
  }
  .header-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .header-label {
    font: 600 10px/1 var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
  }
  .header-sub {
    font: 500 11.5px/1.3 var(--ui);
    color: var(--fg-dim);
    text-transform: none;
    letter-spacing: 0;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .row {
    cursor: default;
  }
  .row-icon {
    margin-right: 6px;
    color: var(--fg-muted);
    display: inline-flex;
    align-items: center;
  }
  .row-icon.dim {
    opacity: 0.65;
  }
  .row-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Hover-only `...` action button. Always visible on active row. */
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
    background: var(--surface-2);
    color: var(--fg);
  }

  .no-match {
    padding: 6px 10px;
    color: var(--fg-faint);
    font-size: 11px;
    font-style: italic;
  }
  .clear-btn {
    width: 16px;
    height: 16px;
    padding: 0;
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

  .spacer {
    flex: 1;
  }

  .footer {
    padding: 8px 10px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg-muted);
    font-size: 11px;
  }
  .avatar {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    background: linear-gradient(135deg, #f59e0b, #b47208);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    color: #1a1102;
  }
  .who {
    flex: 1;
    line-height: 1.2;
  }
  .who-name {
    color: var(--fg);
    font-size: 11.5px;
  }
  .who-sub {
    font-size: 10px;
    color: var(--fg-faint);
  }
</style>
