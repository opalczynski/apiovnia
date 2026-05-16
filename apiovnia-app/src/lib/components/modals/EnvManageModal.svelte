<!--
  EnvManageModal — left-rail env list, right-pane variable editor.
  Master-password sealing (Phase 6) will plug into the "Lock with password"
  button shown next to `requires_unlock` env. For now the button is
  scaffolded (toggle is greyed with a "soon" hint).
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { app } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type { EnvVariable, Environment, EnvironmentId } from "$lib/types/domain";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  /** Env currently being inspected (variables visible on the right). */
  let focused = $state<EnvironmentId | null>(null);

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      dialogEl.showModal();
      // Default focused env: the active one, or the first one available.
      if (!focused) {
        focused = app.activeEnvId ?? app.envs[0]?.id ?? null;
      }
      // Make sure variables are loaded for whatever we focus.
      if (focused) void app.refreshEnvVars(focused);
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  const focusedEnv = $derived<Environment | null>(
    app.envs.find((e) => e.id === focused) ?? null,
  );
  const focusedVars = $derived<EnvVariable[]>(
    focused ? app.envVariablesFor(focused) : [],
  );

  async function newEnv() {
    const name = await dialogs.prompt({
      title: "New environment",
      placeholder: "e.g. dev",
      confirmLabel: "Create environment",
    });
    if (!name) return;
    const created = await app.createEnv(name);
    if (created) {
      focused = created.id;
      await app.refreshEnvVars(created.id);
    }
  }

  async function renameEnv(env: Environment) {
    const next = await dialogs.prompt({
      title: "Rename environment",
      defaultValue: env.name,
      confirmLabel: "Rename",
    });
    if (next && next !== env.name) await app.renameEnv(env.id, next);
  }

  async function deleteEnv(env: Environment) {
    const ok = await dialogs.confirm({
      title: `Delete "${env.name}"?`,
      message:
        "All variables and per-request overrides bound to this env will be removed. Cannot be undone.",
      confirmLabel: "Delete environment",
      danger: true,
    });
    if (!ok) return;
    await app.deleteEnv(env.id);
    if (focused === env.id) focused = app.envs[0]?.id ?? null;
  }

  function focusEnv(id: EnvironmentId) {
    focused = id;
    void app.refreshEnvVars(id);
  }

  // ----- Variable editor ----------------------------------------------------

  // Draft state for the always-empty bottom row.
  let draftName = $state("");
  let draftValue = $state("");

  async function commitDraft() {
    if (!focused) return;
    const name = draftName.trim();
    if (!name) return;
    await app.upsertEnvVariable(focused, name, draftValue, false);
    draftName = "";
    draftValue = "";
  }

  // Per-row inline edit state — keyed by variable name.
  let editing = $state<{ name: string; value: string } | null>(null);

  function startEdit(v: EnvVariable) {
    editing = { name: v.name, value: v.value };
  }
  async function commitEdit() {
    if (!editing || !focused) return;
    const original = focusedVars.find((v) => v.name === editing!.name);
    if (original && original.value !== editing.value) {
      await app.upsertEnvVariable(focused, editing.name, editing.value, original.isSecret);
    }
    editing = null;
  }
  async function deleteVar(v: EnvVariable) {
    if (!focused) return;
    const ok = await dialogs.confirm({
      title: `Delete variable "${v.name}"?`,
      message: "Any `{{...}}` references in your requests will stop resolving.",
      confirmLabel: "Delete variable",
      danger: true,
    });
    if (ok) await app.deleteEnvVariable(focused, v.name);
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
  <div class="card">
    <header class="head">
      <div class="title">Environments</div>
      <div class="msg">
        Define values that get spliced into URLs, headers, body, and auth as
        <code class="mono">{`{{name}}`}</code>. Lock an env with a master password
        in Phase 6 to encrypt its secrets at rest.
      </div>
      <button class="ap-btn icon sm ghost close-x" title="Close (Esc)" onclick={onClose}>
        <Icon d={IC.x} />
      </button>
    </header>

    <div class="body">
      <!-- Left rail: env list -->
      <aside class="rail">
        <div class="rail-head">
          <span>{app.envs.length} env{app.envs.length === 1 ? "" : "s"}</span>
          <button class="ap-btn sm ghost" onclick={newEnv}>
            <Icon d={IC.plus} size={11} /><span>New</span>
          </button>
        </div>
        <div class="rail-list">
          {#if app.envs.length === 0}
            <div class="empty-rail">
              No environments yet. Create one to start overriding requests per env.
            </div>
          {/if}
          {#each app.envs as e (e.id)}
            {@const isFocused = focused === e.id}
            <div
              class="rail-row"
              class:active={isFocused}
              role="button"
              tabindex="0"
              onclick={() => focusEnv(e.id)}
              onkeydown={(ev) => {
                if (ev.key === "Enter") focusEnv(e.id);
              }}
            >
              <span class="rail-name">{e.name}</span>
              {#if e.requiresUnlock}
                <span class="rail-lock"><Icon d={IC.lock} size={10} /></span>
              {/if}
              <button class="ap-btn icon sm ghost rail-act" title="Rename" onclick={(ev) => { ev.stopPropagation(); void renameEnv(e); }}>
                <Icon d={IC.pencil} size={11} />
              </button>
              <button class="ap-btn icon sm ghost rail-act" title="Delete" onclick={(ev) => { ev.stopPropagation(); void deleteEnv(e); }}>
                <Icon d={IC.trash} size={11} />
              </button>
            </div>
          {/each}
        </div>
      </aside>

      <!-- Right pane: variables -->
      <section class="pane">
        {#if !focusedEnv}
          <div class="empty-pane">Pick an environment to edit its variables.</div>
        {:else}
          <div class="pane-head">
            <div class="pane-title">
              Variables in <b>{focusedEnv.name}</b>
            </div>
            <span class="grow"></span>
            <button class="ap-btn sm ghost" disabled title="Master-password sealing arrives in Phase 6">
              <Icon d={IC.lock} size={11} /><span>Lock env… (soon)</span>
            </button>
          </div>

          <div class="vars">
            <div class="vars-head">
              <span class="col-k">Name</span>
              <span class="col-v">Value</span>
              <span class="col-x"></span>
            </div>

            {#if focusedVars.length === 0}
              <div class="vars-empty">
                No variables. Add <code class="mono">{`{{base_url}}`}</code>,
                <code class="mono">{`{{token}}`}</code>, etc. — anything you reference
                in requests.
              </div>
            {/if}

            {#each focusedVars as v (v.name)}
              {@const isEditing = editing?.name === v.name}
              <div class="vars-row">
                <span class="cell-k mono">{v.name}</span>
                {#if isEditing}
                  <input
                    class="cell-v mono input"
                    type="text"
                    bind:value={editing!.value}
                    onblur={() => void commitEdit()}
                    onkeydown={(e) => {
                      if (e.key === "Enter") void commitEdit();
                      else if (e.key === "Escape") editing = null;
                    }}
                  />
                {:else}
                  <button
                    type="button"
                    class="cell-v mono cell-show"
                    onclick={() => startEdit(v)}
                    title="Edit value"
                  >
                    {#if v.value}{v.value}{:else}<em class="empty-val">empty</em>{/if}
                  </button>
                {/if}
                <button
                  type="button"
                  class="ap-btn icon sm ghost vars-kill"
                  title="Delete variable"
                  onclick={() => void deleteVar(v)}
                >
                  <Icon d={IC.x} size={11} />
                </button>
              </div>
            {/each}

            <!-- Draft row -->
            <div class="vars-row draft">
              <input
                class="cell-k mono input"
                type="text"
                placeholder="name"
                bind:value={draftName}
                onkeydown={(e) => {
                  if (e.key === "Enter") void commitDraft();
                }}
              />
              <input
                class="cell-v mono input"
                type="text"
                placeholder="value"
                bind:value={draftValue}
                onkeydown={(e) => {
                  if (e.key === "Enter") void commitDraft();
                }}
              />
              <button
                type="button"
                class="ap-btn icon sm ghost vars-add"
                title="Add variable"
                onclick={() => void commitDraft()}
                disabled={!draftName.trim()}
              >
                <Icon d={IC.plus} size={11} />
              </button>
            </div>
          </div>
        {/if}
      </section>
    </div>

    <footer class="foot">
      <span class="hint">Variables persist immediately to local SQLite.</span>
      <span class="grow"></span>
      <button class="ap-btn" onclick={onClose}>Done <span class="ap-kbd">esc</span></button>
    </footer>
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
    display: grid;
    place-items: center;
  }
  .modal:not([open]) {
    display: none;
  }
  .modal::backdrop {
    background: radial-gradient(circle at 50% 40%, rgba(0, 0, 0, 0.55), rgba(0, 0, 0, 0.85) 70%);
    backdrop-filter: blur(2px);
  }

  .card {
    width: 760px;
    max-width: calc(100vw - 32px);
    height: 540px;
    max-height: calc(100vh - 32px);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--ui);
    position: relative;
  }

  .head {
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border-soft);
    position: relative;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--fg);
  }
  .msg {
    margin-top: 4px;
    font-size: 12px;
    color: var(--fg-muted);
    line-height: 1.5;
  }
  .msg code {
    background: var(--surface-2);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--accent);
  }
  .close-x {
    position: absolute;
    top: 12px;
    right: 12px;
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .rail {
    width: 220px;
    border-right: 1px solid var(--border-soft);
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .rail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    font-size: 10.5px;
    color: var(--fg-faint);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    border-bottom: 1px solid var(--border-soft);
  }
  .rail-list {
    flex: 1;
    overflow: auto;
    padding: 4px;
  }
  .empty-rail {
    padding: 10px 8px;
    font-size: 11px;
    color: var(--fg-faint);
    font-style: italic;
    line-height: 1.5;
  }
  .rail-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--fg-dim);
    font-size: 12px;
  }
  .rail-row:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .rail-row.active {
    background: var(--selected);
    color: var(--fg);
  }
  .rail-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rail-lock {
    color: var(--fg-muted);
    display: inline-flex;
  }
  .rail-act {
    width: 20px;
    height: 20px;
    padding: 0;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .rail-row:hover .rail-act,
  .rail-row.active .rail-act {
    opacity: 1;
  }

  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg);
  }
  .empty-pane {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--fg-faint);
    font-size: 12px;
  }
  .pane-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border-soft);
  }
  .pane-title {
    font-size: 12px;
    color: var(--fg-muted);
  }
  .pane-title b {
    color: var(--fg);
    font-weight: 600;
  }
  .grow {
    flex: 1;
  }

  .vars {
    flex: 1;
    overflow: auto;
  }
  .vars-head {
    display: grid;
    grid-template-columns: 1fr 1.6fr 32px;
    padding: 6px 12px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    border-bottom: 1px solid var(--border-soft);
    font-weight: 600;
  }
  .vars-empty {
    padding: 14px 14px;
    font-size: 11.5px;
    color: var(--fg-faint);
    line-height: 1.6;
  }
  .vars-empty code {
    background: var(--surface-2);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--accent);
  }
  .vars-row {
    display: grid;
    grid-template-columns: 1fr 1.6fr 32px;
    align-items: stretch;
    border-bottom: 1px solid var(--border-soft);
  }
  .cell-k,
  .cell-v {
    height: 30px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    font: 12px/1 var(--mono);
    color: var(--fg);
    background: transparent;
    border: 0;
    outline: none;
    text-align: left;
    cursor: text;
  }
  .cell-show {
    cursor: pointer;
    color: var(--fg);
  }
  .cell-show:hover {
    background: var(--surface-2);
  }
  .input::placeholder {
    color: var(--fg-faint);
    font-style: italic;
  }
  .input:focus {
    background: var(--surface-2);
  }
  .vars-kill,
  .vars-add {
    width: 100%;
    height: 30px;
    padding: 0;
    border-radius: 0;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .vars-row:hover .vars-kill {
    opacity: 1;
  }
  .vars-add {
    opacity: 0.7;
  }
  .vars-row.draft .cell-k,
  .vars-row.draft .cell-v {
    color: var(--fg-faint);
    font-style: italic;
  }
  .vars-row.draft .cell-k:focus,
  .vars-row.draft .cell-v:focus {
    color: var(--fg);
    font-style: normal;
  }

  .foot {
    padding: 10px 16px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-2);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hint {
    font-size: 10.5px;
    color: var(--fg-faint);
  }
  .foot .ap-btn {
    height: 28px;
    padding: 0 14px;
    font-size: 12px;
  }
</style>
