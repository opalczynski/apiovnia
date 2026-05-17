<!--
  CommandPalette — ⌘K / Ctrl+K spotlight-style switcher.

  Scope on open: requests across every collection of the active project +
  collections + every project + active project's envs + a small set of
  "New X…" actions. Cross-project request search is intentionally not
  surfaced — switching to a different project is one Enter away (projects
  are searchable too), and the alternative (loading the entire DB on
  every Cmd-K) doesn't pull its weight at MVP scale.

  Fuzzy ranking: case-insensitive substring with boosts for matches at
  the start of the label, at a word boundary, and shorter labels. Good
  enough for hundreds of items; we'd only need real fzf-style scoring if
  the corpus ever gets to thousands.

  Selection model: arrow keys cycle, Enter runs `current.run()`, Esc /
  click outside closes. Selected item auto-scrolls into view via
  scrollIntoView({block: "nearest"}).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";

  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import MethodBadge from "$lib/components/MethodBadge.svelte";
  import * as ipc from "$lib/api/ipc";
  import { app, SNIPPET_FORMATS, snippetFormatLabel } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type {
    Collection,
    HttpMethod,
    RequestSummary,
  } from "$lib/types/domain";

  type Props = {
    onClose: () => void;
  };

  const { onClose }: Props = $props();

  // -------------------------------------------------------------------------
  // Catalog loading — once per open. Local-only state; the palette is short
  // lived so caching across opens isn't worth the staleness risk.
  // -------------------------------------------------------------------------

  type FlatRequest = {
    request: RequestSummary;
    collection: Collection;
  };

  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let listEl: HTMLDivElement | undefined = $state();
  let query = $state("");
  let cursor = $state(0);
  let loading = $state(true);
  let allCollections = $state<Collection[]>([]);
  let allRequests = $state<FlatRequest[]>([]);

  onMount(() => {
    dialogEl?.showModal();
    queueMicrotask(() => inputEl?.focus());
    void loadCatalog();
  });

  async function loadCatalog(): Promise<void> {
    const pid = app.activeProjectId;
    if (!pid) {
      // Empty active project = palette still useful for project-switch +
      // actions. Skip the request catalog.
      loading = false;
      return;
    }
    try {
      const cols = await ipc.listCollections(pid);
      allCollections = cols;
      // Parallel — typical N=5-15 collections, each <50 requests.
      const lists = await Promise.all(
        cols.map((c) => ipc.listRequests(c.id).then((rs) => ({ c, rs }))),
      );
      allRequests = lists.flatMap(({ c, rs }) =>
        rs.map((r) => ({ request: r, collection: c })),
      );
    } catch (e) {
      // Non-fatal — palette still works with projects + envs + actions.
      // eslint-disable-next-line no-console
      console.warn("[apiovnia] palette catalog load failed", e);
    } finally {
      loading = false;
    }
  }

  // -------------------------------------------------------------------------
  // Items + ranking
  // -------------------------------------------------------------------------

  type Item =
    | {
        kind: "request";
        key: string;
        label: string;
        hint: string;
        method: HttpMethod;
        run: () => void | Promise<void>;
      }
    | {
        kind: "collection";
        key: string;
        label: string;
        hint: string;
        run: () => void | Promise<void>;
      }
    | {
        kind: "project";
        key: string;
        label: string;
        hint: string;
        run: () => void | Promise<void>;
      }
    | {
        kind: "env";
        key: string;
        label: string;
        hint: string;
        run: () => void | Promise<void>;
      }
    | {
        kind: "action";
        key: string;
        label: string;
        hint: string;
        icon: string;
        run: () => void | Promise<void>;
        disabled?: boolean;
        disabledHint?: string;
      };

  /** All candidate items before query filtering. Order here = tie-break
   *  order when scores match. Actions first when query is empty so the
   *  user always sees a "New X…" entry on open. */
  const items = $derived<Item[]>(buildItems());

  function buildItems(): Item[] {
    const out: Item[] = [];
    // Hoisted so the env-actions block below can already gate on them.
    const hasCol = !!app.activeCollectionId;
    const hasProj = !!app.activeProjectId;

    // Requests — flat across active project.
    for (const { request: r, collection: c } of allRequests) {
      const proj = app.projects.find((p) => p.id === c.projectId);
      out.push({
        kind: "request",
        key: `r:${r.id}`,
        label: r.name,
        hint: `${proj?.name ?? "?"} · ${c.name}`,
        method: r.method,
        run: () => {
          if (proj) void app.navigateToRequest(proj.id, c.id, r.id);
          finish();
        },
      });
    }

    // Collections — active project.
    for (const c of allCollections) {
      out.push({
        kind: "collection",
        key: `c:${c.id}`,
        label: c.name,
        hint: "collection",
        run: () => {
          void app.selectCollection(c.id);
          finish();
        },
      });
    }

    // Projects — all of them; switching projects from the palette is the
    // "cross-project request search" workaround.
    for (const p of app.projects) {
      out.push({
        kind: "project",
        key: `p:${p.id}`,
        label: p.name,
        hint: "project",
        run: () => {
          void app.selectProject(p.id);
          finish();
        },
      });
    }

    // Envs — active project. Each env yields:
    //   - "switch to" row (the env itself)
    //   - "enable encryption" action (only when plaintext)
    //   - "disable encryption" action (only when encrypted)
    //   - "lock" action (only when encrypted+unlocked — no new password)
    for (const e of app.envs) {
      const locked = app.isEnvLocked(e.id);
      out.push({
        kind: "env",
        key: `e:${e.id}`,
        label: e.name,
        hint: e.isEncrypted
          ? locked
            ? "environment · locked"
            : "environment · unlocked"
          : "environment",
        run: () => {
          void app.selectEnv(e.id);
          finish();
        },
      });
      if (!e.isEncrypted) {
        out.push({
          kind: "action",
          key: `a:enable-enc:${e.id}`,
          label: `Enable encryption for ${e.name}`,
          hint: "seal with a master password",
          icon: IC.lock,
          run: () => {
            app.openEnvPasswordSetup(e.id);
            finish();
          },
        });
      } else {
        // Encrypted env: offer disable + (if unlocked this session) lock.
        out.push({
          kind: "action",
          key: `a:disable-enc:${e.id}`,
          label: `Disable encryption for ${e.name}`,
          hint: "decrypt back to plaintext (requires current password)",
          icon: IC.unlock,
          run: () => {
            finish();
            void app.disableEncryptionWithPrompt(e.id, e.name);
          },
        });
        if (!locked) {
          out.push({
            kind: "action",
            key: `a:lock:${e.id}`,
            label: `Lock ${e.name}`,
            hint: "drop the session key (no new password)",
            icon: IC.lock,
            run: () => {
              void app.lockEnv(e.id);
              finish();
            },
          });
        }
      }
    }

    // Manage envs — opens the EnvManageModal.
    out.push({
      kind: "action",
      key: "a:manage-envs",
      label: "Manage envs & variables",
      hint: hasProj ? "open env manager" : "pick a project first",
      icon: IC.settings,
      disabled: !hasProj,
      disabledHint: "No active project",
      run: () => {
        app.openEnvManage();
        finish();
      },
    });

    // OpenAPI import / export — gate on active project + collection.
    const activeProj = app.activeProject;
    if (activeProj) {
      out.push({
        kind: "action",
        key: "a:openapi-import",
        label: `Import OpenAPI into ${activeProj.name}`,
        hint: "open a YAML/JSON file",
        icon: IC.arrowR,
        run: () => {
          finish();
          void (async () => {
            const path = await open({
              title: `Import OpenAPI into "${activeProj.name}"`,
              multiple: false,
              directory: false,
              filters: [
                { name: "OpenAPI", extensions: ["yaml", "yml", "json"] },
              ],
            });
            if (typeof path === "string") {
              await app.importOpenapiForProject(activeProj.id, path);
            }
          })();
        },
      });
    }
    const activeColl = app.activeCollection;
    if (activeColl) {
      out.push({
        kind: "action",
        key: "a:openapi-export",
        label: `Export ${activeColl.name} as OpenAPI`,
        hint: "secrets stripped to placeholders",
        icon: IC.send,
        run: () => {
          finish();
          void app.exportCollectionInteractive(activeColl.id);
        },
      });
    }

    // Actions — bottom by default; ranked normally when matched.

    // Contextual: copy the *currently selected* request as one of the
    // five snippet formats. One palette entry per format so power users
    // can `⌘K → "python" → ↵` without going through the right-click menu.
    // Disabled when nothing's selected — entries stay visible so users
    // discover them.
    const activeReq = app.activeRequest;
    for (const fmt of SNIPPET_FORMATS) {
      const formatLabel = snippetFormatLabel(fmt);
      out.push({
        kind: "action",
        key: `a:copy-${fmt}`,
        label: activeReq
          ? `Copy as ${formatLabel}: ${activeReq.name}`
          : `Copy as ${formatLabel}`,
        hint: activeReq
          ? "resolved against active env"
          : "pick a request first",
        icon: IC.copy,
        disabled: !activeReq,
        disabledHint: "No active request",
        run: () => {
          finish();
          if (activeReq) void app.copyRequestAsSnippet(activeReq.id, fmt);
        },
      });
    }

    out.push({
      kind: "action",
      key: "a:new-req",
      label: "New request",
      hint: hasCol ? "in current collection" : "pick a collection first",
      icon: IC.plus,
      disabled: !hasCol,
      disabledHint: "No active collection",
      run: async () => {
        finish();
        const name = await dialogs.prompt({
          title: "New request",
          placeholder: "e.g. Get user",
          confirmLabel: "Create request",
        });
        if (name) await app.createRequest(name);
      },
    });
    out.push({
      kind: "action",
      key: "a:new-col",
      label: "New collection",
      hint: hasProj ? "in current project" : "pick a project first",
      icon: IC.collection,
      disabled: !hasProj,
      disabledHint: "No active project",
      run: async () => {
        finish();
        const name = await dialogs.prompt({
          title: "New collection",
          placeholder: "e.g. Auth",
          confirmLabel: "Create collection",
        });
        if (name) await app.createCollection(name);
      },
    });
    out.push({
      kind: "action",
      key: "a:new-proj",
      label: "New project",
      hint: "",
      icon: IC.folder,
      run: async () => {
        finish();
        const name = await dialogs.prompt({
          title: "New project",
          placeholder: "e.g. My API",
          confirmLabel: "Create project",
        });
        if (name) await app.createProject(name);
      },
    });
    out.push({
      kind: "action",
      key: "a:new-env",
      label: "New environment",
      hint: hasProj ? "in current project" : "pick a project first",
      icon: IC.globe,
      disabled: !hasProj,
      disabledHint: "No active project",
      run: async () => {
        finish();
        const name = await dialogs.prompt({
          title: "New environment",
          placeholder: "e.g. dev",
          confirmLabel: "Create environment",
        });
        if (name) await app.createEnv(name);
      },
    });
    return out;
  }

  /** Returns `null` for non-matches (filter out) or a positive integer
   *  score otherwise. Higher = better. */
  function score(label: string, hint: string, q: string): number | null {
    if (!q) return 1; // empty query — everything is a candidate, keep input order
    const ql = q.toLowerCase();
    const ll = label.toLowerCase();
    const hl = hint.toLowerCase();
    let s: number | null = null;
    if (ll.startsWith(ql)) s = 1000;
    else {
      const i = ll.indexOf(ql);
      if (i !== -1) {
        const wordStart = i === 0 || /[\s/_\-.]/.test(ll[i - 1] ?? "");
        s = (wordStart ? 600 : 300) - i;
      }
    }
    // Hint match is a weak signal — still beats no match.
    if (s === null && hl.includes(ql)) s = 50;
    if (s === null) return null;
    // Shorter labels rank slightly higher to break ties.
    s += Math.max(0, 80 - ll.length);
    return s;
  }

  const ranked = $derived<Item[]>(rank());

  function rank(): Item[] {
    const q = query.trim();
    const scored: { item: Item; s: number }[] = [];
    for (const it of items) {
      const s = score(it.label, it.hint, q);
      if (s !== null) scored.push({ item: it, s });
    }
    // Stable-ish sort: by score desc, then by kind priority, then alpha.
    const kindRank = { request: 0, collection: 1, env: 2, project: 3, action: 4 };
    scored.sort((a, b) => {
      if (b.s !== a.s) return b.s - a.s;
      if (kindRank[a.item.kind] !== kindRank[b.item.kind]) {
        return kindRank[a.item.kind] - kindRank[b.item.kind];
      }
      return a.item.label.localeCompare(b.item.label);
    });
    return scored.map((x) => x.item);
  }

  // Reset cursor to the top whenever the candidate set changes, so the
  // highlight doesn't dangle on a stale row. We only write to `cursor`
  // here (no read), so this can't self-trigger.
  $effect(() => {
    void ranked.length;
    cursor = 0;
  });

  // -------------------------------------------------------------------------
  // Keyboard navigation
  // -------------------------------------------------------------------------

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      cursor = Math.min(cursor + 1, ranked.length - 1);
      scrollCursorIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      cursor = Math.max(cursor - 1, 0);
      scrollCursorIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      runCursor();
    }
    // Esc handled by <dialog>'s native cancel → onCloseEvt.
  }

  function scrollCursorIntoView() {
    queueMicrotask(() => {
      const el = listEl?.querySelector<HTMLElement>(`[data-i="${cursor}"]`);
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  async function runCursor() {
    const it = ranked[cursor];
    if (!it) return;
    if (it.kind === "action" && it.disabled) return;
    await it.run();
  }

  function finish() {
    onClose();
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onClose();
  }
</script>

<dialog
  bind:this={dialogEl}
  class="modal"
  oncancel={(e) => {
    e.preventDefault();
    onClose();
  }}
  onclick={onBackdropClick}
>
  <div class="card" onkeydown={onKeydown} role="presentation">
    <div class="search">
      <span class="search-icon"><Icon d={IC.search} size={14} /></span>
      <input
        bind:this={inputEl}
        bind:value={query}
        class="search-input"
        type="text"
        placeholder="Jump to a request, collection, project, env…"
        autocomplete="off"
        spellcheck="false"
      />
      <span class="search-kbd">esc</span>
    </div>

    <div bind:this={listEl} class="list" role="listbox">
      {#if loading}
        <div class="status">Loading catalog…</div>
      {:else if ranked.length === 0}
        <div class="status">
          No matches for <b>"{query}"</b>.
          <div class="status-hint">Tip: type the name of a request, project, env, or "new"…</div>
        </div>
      {/if}

      {#each ranked as it, i (it.key)}
        {@const active = i === cursor}
        {@const isAction = it.kind === "action"}
        {@const disabled = isAction && it.disabled === true}
        <button
          type="button"
          class="row"
          class:active
          class:disabled
          data-i={i}
          role="option"
          aria-selected={active}
          onmouseenter={() => (cursor = i)}
          onclick={() => void runCursor()}
          title={disabled ? it.disabledHint ?? "" : ""}
        >
          <span class="row-icon">
            {#if it.kind === "request"}
              <MethodBadge method={it.method} />
            {:else if isAction}
              <Icon d={it.icon} size={13} />
            {:else if it.kind === "collection"}
              <Icon d={IC.collection} size={13} />
            {:else if it.kind === "project"}
              <Icon d={IC.folder} size={13} />
            {:else if it.kind === "env"}
              <Icon d={IC.globe} size={13} />
            {/if}
          </span>
          <span class="row-label">{it.label}</span>
          {#if it.hint}
            <span class="row-hint">{it.hint}</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="foot">
      <span class="foot-hint"><span class="ap-kbd">↑↓</span> navigate</span>
      <span class="foot-hint"><span class="ap-kbd">↵</span> select</span>
      <span class="foot-hint"><span class="ap-kbd">esc</span> close</span>
      <div class="grow"></div>
      <span class="foot-count">{ranked.length} item{ranked.length === 1 ? "" : "s"}</span>
    </div>
  </div>
</dialog>

<style>
  .modal {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    max-width: 100vw;
    max-height: 100vh;
    margin: 0;
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--fg);
    /* Spotlight-style: card sits in the top third of the viewport, not
       dead-centre, because that's where eyes naturally land on a fresh
       open. macOS Spotlight + Raycast both do this. */
    display: grid;
    place-items: start center;
    padding-top: 14vh;
  }
  .modal:not([open]) {
    display: none;
  }
  .modal::backdrop {
    background: radial-gradient(
      circle at 50% 30%,
      rgba(0, 0, 0, 0.5),
      rgba(0, 0, 0, 0.78) 70%
    );
    backdrop-filter: blur(2px);
  }

  .card {
    width: 640px;
    max-width: calc(100vw - 32px);
    max-height: 60vh;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(245, 158, 11, 0.06);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--ui);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-soft);
  }
  .search-icon {
    color: var(--fg-muted);
    display: inline-flex;
  }
  .search-input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--fg);
    font-size: 14.5px;
    font-family: var(--ui);
  }
  .search-input::placeholder {
    color: var(--fg-faint);
  }
  .search-kbd {
    font-family: var(--mono);
    font-size: 10px;
    padding: 2px 6px;
    color: var(--fg-faint);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 4px;
    min-height: 80px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--fg-dim);
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    font-family: var(--ui);
  }
  .row.active {
    background: var(--selected);
    color: var(--fg);
  }
  .row.disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .row-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    color: var(--fg-muted);
  }
  .row.active .row-icon {
    color: var(--fg-dim);
  }
  .row-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.active .row-label {
    font-weight: 600;
  }
  .row-hint {
    font-size: 11px;
    color: var(--fg-faint);
    font-family: var(--mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 45%;
  }

  .status {
    padding: 22px 16px;
    text-align: center;
    color: var(--fg-muted);
    font-size: 12.5px;
  }
  .status b {
    color: var(--fg);
  }
  .status-hint {
    margin-top: 6px;
    font-size: 11px;
    color: var(--fg-faint);
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 14px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-2);
    font-size: 11px;
    color: var(--fg-faint);
  }
  .foot-hint {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .grow {
    flex: 1;
  }
  .foot-count {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--fg-muted);
  }
</style>
