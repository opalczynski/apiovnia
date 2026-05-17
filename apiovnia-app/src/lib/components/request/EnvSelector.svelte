<!--
  EnvSelector — pill in the UrlBar that picks the active environment for
  the current project. The dropdown also exposes "+ New environment" and
  "Manage envs…", so the user never has to go hunting in a settings page.

  Color dot heuristic mirrors the design canvas:
    prod / production    → red
    stage / staging      → yellow
    dev / development /
    local                → green
    anything else        → accent (amber)
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import Popover from "$lib/components/ui/Popover.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type { Environment, EnvironmentId } from "$lib/types/domain";

  type Props = {
    /** Called to open the env management modal — owned by the parent. */
    onManage: () => void;
  };

  const { onManage }: Props = $props();

  let btnEl: HTMLButtonElement | undefined = $state();
  let menuOpen = $state(false);

  function envColor(name: string): string {
    const n = name.toLowerCase();
    if (n === "prod" || n === "production") return "var(--err)";
    if (n === "stage" || n === "staging") return "var(--warn)";
    if (n === "dev" || n === "development" || n.startsWith("local")) return "var(--ok)";
    return "var(--accent)";
  }

  function pick(id: EnvironmentId | null) {
    menuOpen = false;
    void app.selectEnv(id);
  }

  function unlock(id: EnvironmentId) {
    menuOpen = false;
    app.promptUnlock(id);
  }

  function lock(id: EnvironmentId) {
    menuOpen = false;
    void app.lockEnv(id);
  }

  async function newEnv() {
    menuOpen = false;
    const name = await dialogs.prompt({
      title: "New environment",
      message: "Common names: dev, stage, prod, local. You can rename later.",
      placeholder: "e.g. dev",
      confirmLabel: "Create environment",
    });
    if (name) await app.createEnv(name);
  }

  function manage() {
    menuOpen = false;
    onManage();
  }

  function dotColor(env: Environment | null): string {
    if (!env) return "var(--fg-muted)";
    if (env.isEncrypted && app.isEnvLocked(env.id)) return "var(--fg-faint)";
    return envColor(env.name);
  }

  const active = $derived(app.activeEnv);
  const activeLocked = $derived<boolean>(
    active ? app.isEnvLocked(active.id) : false,
  );
</script>

<button
  bind:this={btnEl}
  class="env-pick"
  class:has-env={active != null}
  onclick={() => (menuOpen = !menuOpen)}
  disabled={!app.activeProjectId}
  title={app.activeProjectId
    ? active
      ? activeLocked
        ? `${active.name} — locked. Click to unlock.`
        : `Active environment: ${active.name}`
      : "No environment selected — request uses base values"
    : "Pick a project to manage environments"}
>
  <span class="env-dot" style:background={dotColor(active)}></span>
  <span class="env-name">{active?.name ?? "no env"}</span>
  {#if active?.isEncrypted}
    <span class="muted" title={activeLocked ? "Locked" : "Unlocked this session"}>
      <Icon d={activeLocked ? IC.lock : IC.unlock} size={12} />
    </span>
  {/if}
  <span class="muted"><Icon d={IC.caret} size={12} /></span>
</button>

<Popover anchor={btnEl} bind:open={menuOpen} placement="bottom-end">
  <div class="menu">
    <div class="menu-head">Environment</div>

    <button class="menu-row" class:active={active == null} onclick={() => pick(null)}>
      <span class="env-dot" style:background="var(--fg-muted)"></span>
      <span class="row-label">No environment</span>
      <span class="row-sub">use base values</span>
      {#if active == null}
        <span class="row-check"><Icon d={IC.check} size={11} /></span>
      {/if}
    </button>

    {#if app.envs.length > 0}
      <div class="menu-sep"></div>
    {/if}

    {#each app.envs as e (e.id)}
      {@const isActive = active?.id === e.id}
      {@const locked = app.isEnvLocked(e.id)}
      <div class="row-wrap" class:active={isActive}>
        <button class="menu-row" onclick={() => pick(e.id)}>
          <span class="env-dot" style:background={dotColor(e)}></span>
          <span class="row-label">{e.name}</span>
          {#if e.isEncrypted}
            <span
              class="row-lock"
              title={locked ? "Encrypted — locked" : "Encrypted — unlocked this session"}
            >
              <Icon d={locked ? IC.lock : IC.unlock} size={11} />
            </span>
          {/if}
          {#if isActive}
            <span class="row-check"><Icon d={IC.check} size={11} /></span>
          {/if}
        </button>
        {#if e.isEncrypted}
          {#if locked}
            <button
              class="row-action"
              title="Unlock with master password"
              onclick={(ev) => {
                ev.stopPropagation();
                unlock(e.id);
              }}
            >
              <Icon d={IC.unlock} size={11} />
            </button>
          {:else}
            <button
              class="row-action"
              title="Lock (drop session key)"
              onclick={(ev) => {
                ev.stopPropagation();
                lock(e.id);
              }}
            >
              <Icon d={IC.lock} size={11} />
            </button>
          {/if}
        {/if}
      </div>
    {/each}

    <div class="menu-sep"></div>

    <button class="menu-row plain" onclick={newEnv}>
      <span class="row-icon"><Icon d={IC.plus} size={12} /></span>
      <span class="row-label">New environment…</span>
    </button>
    <button class="menu-row plain" onclick={manage} disabled={app.envs.length === 0}>
      <span class="row-icon"><Icon d={IC.settings} size={12} /></span>
      <span class="row-label">Manage envs &amp; variables…</span>
    </button>
  </div>
</Popover>

<style>
  .env-pick {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-dim);
    font-size: 12px;
    cursor: pointer;
    transition:
      border-color 0.12s,
      color 0.12s,
      background 0.12s;
  }
  .env-pick:hover:not(:disabled) {
    border-color: var(--border-strong);
    color: var(--fg);
  }
  .env-pick.has-env {
    color: var(--fg);
  }
  .env-pick:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .env-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .env-name {
    font-weight: 500;
  }
  .muted {
    color: var(--fg-muted);
    display: inline-flex;
    align-items: center;
  }

  /* Dropdown */
  .menu {
    min-width: 240px;
    padding: 4px;
  }
  .menu-head {
    padding: 6px 10px 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    font-weight: 600;
  }
  .menu-sep {
    height: 1px;
    background: var(--border-soft);
    margin: 4px 0;
  }
  .menu-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    border-radius: 5px;
    color: var(--fg-dim);
    cursor: pointer;
    text-align: left;
    font-size: 12px;
  }
  .menu-row:hover:not(:disabled) {
    background: var(--hover);
    color: var(--fg);
  }
  .menu-row:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
  .menu-row.active {
    color: var(--fg);
  }
  .menu-row.active .row-label {
    font-weight: 600;
  }
  .row-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-sub {
    font-size: 10.5px;
    color: var(--fg-faint);
    font-style: italic;
  }
  .row-check {
    color: var(--accent);
    display: inline-flex;
  }
  .row-lock {
    color: var(--fg-muted);
    display: inline-flex;
  }
  .menu-row.plain {
    color: var(--fg-muted);
  }
  .row-icon {
    display: inline-flex;
    color: var(--fg-muted);
  }

  /* Row + side action button (unlock / lock) — match menu-row sizing. */
  .row-wrap {
    display: flex;
    align-items: stretch;
    border-radius: 5px;
  }
  .row-wrap:hover {
    background: var(--hover);
  }
  .row-wrap.active {
    background: transparent;
  }
  .row-wrap .menu-row {
    flex: 1;
  }
  .row-wrap:hover .menu-row,
  .row-wrap.active .menu-row {
    background: transparent;
  }
  .row-wrap.active .menu-row {
    color: var(--fg);
  }
  .row-wrap.active .row-label {
    font-weight: 600;
  }
  .row-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    border-radius: 5px;
    opacity: 0;
    transition: opacity 0.12s, color 0.12s, background 0.12s;
  }
  .row-wrap:hover .row-action {
    opacity: 1;
  }
  .row-action:hover {
    color: var(--accent);
    background: var(--surface-2);
  }
</style>
