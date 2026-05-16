<!--
  DetailPanel — full request editor (URL bar + tabs) on top, response viewer
  placeholder below the horizontal splitter. Response wiring lands in Phase 3.

  Selecting a different request flushes any pending debounced save first
  (handled in the store's `selectRequest`); this component just renders the
  current `app.activeRequest`.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import Resizer from "$lib/components/layout/Resizer.svelte";
  import Tabs, { type TabSpec } from "$lib/components/ui/Tabs.svelte";
  import UrlBar from "$lib/components/request/UrlBar.svelte";
  import ParamsTab from "$lib/components/request/tabs/ParamsTab.svelte";
  import HeadersTab from "$lib/components/request/tabs/HeadersTab.svelte";
  import BodyTab from "$lib/components/request/tabs/BodyTab.svelte";
  import AuthTab from "$lib/components/request/tabs/AuthTab.svelte";
  import EnvOverridesTab from "$lib/components/request/tabs/EnvOverridesTab.svelte";
  import ResponseViewer from "$lib/components/response/ResponseViewer.svelte";
  import EnvManageModal from "$lib/components/modals/EnvManageModal.svelte";
  import { panels } from "$lib/stores/panels.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type { Request } from "$lib/types/domain";

  type TabId = "params" | "headers" | "body" | "auth" | "env" | "tests";
  let activeTab = $state<TabId>("body");
  let envManageOpen = $state(false);

  // Mirror panel CTAs — same prompt flows the left/middle panels use, kept
  // here so the user can create directly from the right pane without
  // chasing the `+` buttons.

  async function newProjectFromEmptyState() {
    const name = await dialogs.prompt({
      title: "New project",
      message: "Projects are top-level groupings — pick a workspace name.",
      placeholder: "e.g. UDL, Resi4Rent…",
      confirmLabel: "Create project",
    });
    if (name) await app.createProject(name);
  }

  async function newCollectionFromEmptyState() {
    if (!app.activeProjectId) return;
    const name = await dialogs.prompt({
      title: "New collection",
      message: `In project: ${app.activeProject?.name ?? ""}`,
      placeholder: "e.g. Auth, Properties…",
      confirmLabel: "Create collection",
    });
    if (name) await app.createCollection(name);
  }

  async function newRequestFromEmptyState() {
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

  let containerEl: HTMLDivElement | undefined = $state();

  function adjustSplit(delta: number) {
    if (!containerEl) return;
    const totalH = containerEl.getBoundingClientRect().height;
    if (totalH <= 0) return;
    panels.requestSplit = panels.requestSplit + delta / totalH;
  }

  function patch(p: Partial<Request>) {
    app.updateActiveRequest(p);
  }

  // -----------------------------------------------------------------------
  // Tab specs — counts/badges derived from the live request.
  // -----------------------------------------------------------------------

  function activeCount(rows: { enabled: boolean }[]): number {
    return rows.filter((r) => r.enabled).length;
  }

  function overrideFieldCount(): number {
    const o = app.activeOverride;
    if (!o) return 0;
    let n = 0;
    if (o.method != null) n++;
    if (o.url != null) n++;
    if (o.headers != null) n++;
    if (o.params != null) n++;
    if (o.bodyType != null) n++;
    if (o.bodyContent != null) n++;
    if (o.auth != null) n++;
    return n;
  }

  let tabSpecs = $derived.by<TabSpec<TabId>[]>(() => {
    const r = app.activeRequest;
    if (!r) return [];
    const paramsCount = activeCount(r.params);
    const headersCount = activeCount(r.headers);
    const envCount = overrideFieldCount();
    return [
      {
        id: "params",
        label: "Params",
        count: paramsCount > 0 ? paramsCount : null,
      },
      {
        id: "headers",
        label: "Headers",
        count: headersCount > 0 ? headersCount : null,
      },
      {
        id: "body",
        label: "Body",
        badge: r.bodyType === "none" ? undefined : r.bodyType,
      },
      {
        id: "auth",
        label: "Auth",
        badge: r.auth.type === "none" ? undefined : r.auth.type,
      },
      {
        id: "env",
        label: "Env Overrides",
        count: envCount > 0 ? envCount : null,
      },
      { id: "tests", label: "Tests", soon: true, disabled: true },
    ];
  });

  // -----------------------------------------------------------------------
  // Save indicator labels — driven by the 4-state machine in the store.
  // -----------------------------------------------------------------------

  let saveLabel = $derived.by(() => {
    switch (app.saveState) {
      case "editing":
        return "editing";
      case "saving":
        return "saving…";
      case "saved":
        return "saved";
      case "idle":
      default:
        return null; // render nothing when nothing's happening
    }
  });
</script>

<div class="detail" bind:this={containerEl}>
  {#if !app.activeRequest}
    {#if !app.activeProjectId}
      <!-- Cold start: no project at all. Funnel to project create first. -->
      <div class="placeholder cta">
        <Icon d={IC.folder} size={22} class="hero-icon" />
        <div class="cta-title"><strong>Welcome to Apiovnia.</strong></div>
        <div class="cta-msg">
          Projects group related collections (typically one per API or workspace).
          Create one to get started.
        </div>
        <button class="ap-btn cta-btn" onclick={() => void newProjectFromEmptyState()}>
          <Icon d={IC.plus} />
          <span>New project</span>
        </button>
      </div>
    {:else if !app.activeCollectionId}
      <!-- Project selected, but no collections yet. -->
      <div class="placeholder cta">
        <Icon d={IC.collection} size={22} class="hero-icon" />
        <div class="cta-title">
          No collections in <strong>{app.activeProject?.name ?? "this project"}</strong>
        </div>
        <div class="cta-msg">
          Collections bundle related requests — e.g. <em>Auth</em>, <em>Users</em>,
          <em>Properties</em>. You'll add requests inside them.
        </div>
        <button class="ap-btn cta-btn" onclick={() => void newCollectionFromEmptyState()}>
          <Icon d={IC.plus} />
          <span>New collection</span>
        </button>
      </div>
    {:else if app.requests.length === 0}
      <!-- Collection selected but empty — same CTA pattern. -->
      <div class="placeholder cta">
        <Icon d={IC.send} size={22} class="hero-icon" />
        <div class="cta-title">
          No requests in <strong>{app.activeCollection?.name ?? "this collection"}</strong>
        </div>
        <div class="cta-msg">
          Create your first one to start hitting endpoints. You can rename and
          duplicate later.
        </div>
        <button class="ap-btn cta-btn" onclick={() => void newRequestFromEmptyState()}>
          <Icon d={IC.plus} />
          <span>New request</span>
        </button>
      </div>
    {:else}
      <!-- Requests exist but none selected (rare — cascade auto-picks first). -->
      <div class="placeholder">
        <Icon d={IC.arrowR} size={18} />
        <div>
          <strong>Pick a request</strong> from the middle panel.
        </div>
      </div>
    {/if}
  {:else}
    {@const r = app.activeRequest}
    <UrlBar request={r} onPatch={patch} onManageEnvs={() => (envManageOpen = true)} />

    <Tabs value={activeTab} tabs={tabSpecs} onChange={(v) => (activeTab = v)}>
      <div class="tab-area" style="flex: {panels.requestSplit};">
        {#if activeTab === "params"}
          <ParamsTab request={r} onPatch={patch} />
        {:else if activeTab === "headers"}
          <HeadersTab request={r} onPatch={patch} />
        {:else if activeTab === "body"}
          <BodyTab request={r} onPatch={patch} />
        {:else if activeTab === "auth"}
          <AuthTab request={r} onPatch={patch} />
        {:else if activeTab === "env"}
          <EnvOverridesTab request={r} onManageEnvs={() => (envManageOpen = true)} />
        {/if}
      </div>
    </Tabs>

    <Resizer orientation="horizontal" onDrag={adjustSplit} />

    <div class="resp-wrap" style="flex: {1 - panels.requestSplit};">
      <ResponseViewer />
    </div>

    <footer class="status">
      {#if saveLabel}
        <span class="status-cell save-{app.saveState}">
          <span class="status-dot"></span>
          {saveLabel}
        </span>
      {:else}
        <span class="status-cell idle">
          <span class="status-dot"></span>
        </span>
      {/if}
      <span class="grow"></span>
      <span class="status-cell mono">{r.id.slice(0, 16)}</span>
    </footer>
  {/if}
</div>

<EnvManageModal open={envManageOpen} onClose={() => (envManageOpen = false)} />

<style>
  .detail {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    background: var(--bg);
    height: 100%;
  }

  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--fg-muted);
    font-size: 13px;
    padding: 24px;
    text-align: center;
  }
  .placeholder strong {
    color: var(--fg);
    font-weight: 600;
  }
  /* Active-collection-but-empty variant — more inviting, real CTA button.
     Scope the amber tint to the hero icon only — without the class selector
     it leaks into the CTA button's plus icon and renders it invisible
     against the amber button background. */
  .placeholder.cta :global(.hero-icon) {
    color: var(--accent);
    opacity: 0.7;
  }
  .cta-title {
    font-size: 14px;
    color: var(--fg-dim);
    margin-top: 4px;
  }
  .cta-title strong {
    color: var(--fg);
    font-weight: 600;
  }
  .cta-msg {
    font-size: 12px;
    color: var(--fg-faint);
    max-width: 360px;
    line-height: 1.5;
  }
  .cta-btn {
    margin-top: 8px;
    height: 32px;
    padding: 0 18px;
    background: linear-gradient(180deg, #f5a623 0%, #e08f0b 100%);
    border-color: #c57e07;
    color: #1a1102;
    font-weight: 600;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.2),
      0 1px 0 rgba(255, 255, 255, 0.18) inset;
  }
  .cta-btn:hover {
    background: linear-gradient(180deg, #ffb13b 0%, #ed9a12 100%);
  }

  .tab-area {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
  }

  .resp-wrap {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--surface);
  }

  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 12px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    font-size: 10.5px;
    color: var(--fg-muted);
  }
  .status-cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--fg-faint);
    transition:
      background 0.25s,
      box-shadow 0.25s;
  }
  /* Editing — amber, steady. User just typed, debounce window is open. */
  .save-editing .status-dot {
    background: var(--accent);
  }
  .save-editing {
    color: var(--accent);
  }
  /* Saving — amber, pulsing while the IPC roundtrip is in flight. */
  .save-saving .status-dot {
    background: var(--accent);
    animation: pulse 0.9s ease-in-out infinite;
  }
  .save-saving {
    color: var(--accent);
  }
  /* Saved — green flash with a soft halo, fades back to idle after 800ms. */
  .save-saved .status-dot {
    background: var(--ok);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ok) 25%, transparent);
  }
  .save-saved {
    color: var(--ok);
  }
  /* Idle — neutral grey, no label. */
  .idle .status-dot {
    background: var(--fg-faint);
    opacity: 0.6;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
  .grow {
    flex: 1;
  }
</style>
