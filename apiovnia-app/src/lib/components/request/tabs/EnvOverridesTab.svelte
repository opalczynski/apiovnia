<!--
  EnvOverridesTab — per-`(request, activeEnv)` patch editor.

  Each field shows a toggle + (when on) an editor + the base value greyed
  below. Toggling off clears the override (sets it to `null`) and the
  request falls back to the base value for that field.

  The store debounces writes — same 250 ms cadence as the request editor.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import MethodBadge from "$lib/components/MethodBadge.svelte";
  import Select, { type SelectOption } from "$lib/components/ui/Select.svelte";
  import Segment from "$lib/components/ui/Segment.svelte";
  import KeyValueTable from "$lib/components/request/KeyValueTable.svelte";
  import CodeMirrorEditor from "$lib/components/ui/CodeMirrorEditor.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import type {
    AuthConfig,
    BodyType,
    HttpMethod,
    KeyValue,
    Request,
  } from "$lib/types/domain";

  type Props = {
    request: Request;
    onManageEnvs: () => void;
  };

  const { request, onManageEnvs }: Props = $props();

  const env = $derived(app.activeEnv);
  const ovr = $derived(app.activeOverride);

  // Helpers for the small "field row" pattern -------------------------------

  function patch<T extends keyof OverridableMap>(field: T, value: OverridableMap[T] | null) {
    app.updateActiveOverride({ [field]: value } as never);
  }

  type OverridableMap = {
    method: HttpMethod;
    url: string;
    headers: KeyValue[];
    params: KeyValue[];
    bodyType: BodyType;
    bodyContent: string;
    auth: AuthConfig;
  };

  async function resetAll() {
    if (!env) return;
    const ok = await dialogs.confirm({
      title: `Reset all overrides in "${env.name}"?`,
      message: "This request will fall back to its base values whenever this env is active.",
      confirmLabel: "Reset overrides",
      danger: true,
    });
    if (ok) await app.resetActiveOverride();
  }

  // Method dropdown options.
  const METHODS: SelectOption<HttpMethod>[] = [
    { value: "GET", label: "GET" },
    { value: "POST", label: "POST" },
    { value: "PUT", label: "PUT" },
    { value: "PATCH", label: "PATCH" },
    { value: "DELETE", label: "DELETE" },
    { value: "HEAD", label: "HEAD" },
    { value: "OPTIONS", label: "OPTIONS" },
  ];

  const BODY_TYPES: { value: BodyType; label: string }[] = [
    { value: "none", label: "None" },
    { value: "json", label: "JSON" },
    { value: "form", label: "Form" },
    { value: "raw", label: "Raw" },
  ];

  const AUTH_TYPES: SelectOption<AuthConfig["type"]>[] = [
    { value: "none", label: "None" },
    { value: "bearer", label: "Bearer" },
    { value: "basic", label: "Basic" },
    { value: "apikey", label: "API Key" },
  ];

  // Default values when the user flips a field "on".
  function defaultForAuth(t: AuthConfig["type"]): AuthConfig {
    switch (t) {
      case "none":
        return { type: "none" };
      case "bearer":
        return { type: "bearer", token: "" };
      case "basic":
        return { type: "basic", username: "", password: "" };
      case "apikey":
        return { type: "apikey", name: "", value: "", in: "header" };
    }
  }
</script>

{#if app.envs.length === 0}
  <div class="empty">
    <div class="empty-title">No environments yet</div>
    <div class="empty-msg">
      Create an environment to override request fields per stage —
      <code class="mono">dev</code>, <code class="mono">stage</code>,
      <code class="mono">prod</code> are the usual suspects.
    </div>
    <button class="ap-btn cta" onclick={onManageEnvs}>
      <Icon d={IC.plus} /> <span>New environment…</span>
    </button>
  </div>
{:else if !env}
  <div class="empty">
    <div class="empty-title">No environment selected</div>
    <div class="empty-msg">
      Pick one in the top-right env switcher to start editing overrides for
      this request.
    </div>
  </div>
{:else if env.isEncrypted && app.isEnvLocked(env.id)}
  <div class="empty">
    <div class="lock-glow">
      <Icon d={IC.lock} size={18} />
    </div>
    <div class="empty-title">{env.name} is locked</div>
    <div class="empty-msg">
      Override values for this env are encrypted at rest. Unlock with your
      master password to view and edit them.
    </div>
    <button class="ap-btn cta" onclick={() => app.promptUnlock(env.id)}>
      <Icon d={IC.unlock} /><span>Unlock {env.name}</span>
    </button>
  </div>
{:else}
  <div class="root">
    <div class="bar">
      <Icon d={IC.globe} size={13} class="bar-icon" />
      <div class="bar-msg">
        Patches on this request when <b>{env.name}</b> is active.
        Unset fields inherit the base. Resolution order:
        <span class="mono">request &gt; env override &gt; base</span>.
      </div>
      <span class="grow"></span>
      <button class="ap-btn sm ghost" onclick={resetAll} disabled={ovr == null}>
        Reset all in {env.name}
      </button>
    </div>

    <div class="fields">
      <!-- URL -->
      <div class="field" class:on={ovr?.url != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.url != null}
              onchange={(e) => patch("url", (e.currentTarget as HTMLInputElement).checked ? request.url : null)}
            />
            <span class="dot"></span>
            <span class="lbl">URL</span>
          </label>
        </div>
        {#if ovr?.url != null}
          <input
            class="cm-shell mono"
            type="text"
            placeholder="override URL for {env.name}"
            value={ovr.url}
            oninput={(e) => patch("url", (e.currentTarget as HTMLInputElement).value)}
          />
        {/if}
        <div class="base mono"><span class="base-tag">base</span>{request.url || "—"}</div>
      </div>

      <!-- Method -->
      <div class="field" class:on={ovr?.method != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.method != null}
              onchange={(e) => patch("method", (e.currentTarget as HTMLInputElement).checked ? request.method : null)}
            />
            <span class="dot"></span>
            <span class="lbl">Method</span>
          </label>
        </div>
        {#if ovr?.method != null}
          <div class="method-row">
            <Select
              value={ovr.method}
              options={METHODS}
              onChange={(m) => patch("method", m)}
              ariaLabel="Override HTTP method"
            >
              {#snippet triggerLabel(o)}
                <MethodBadge method={o.value} />
              {/snippet}
              {#snippet optionLabel(o)}
                <MethodBadge method={o.value} />
                <span class="opt-label mono">{o.label}</span>
              {/snippet}
            </Select>
          </div>
        {/if}
        <div class="base">
          <span class="base-tag">base</span><MethodBadge method={request.method} />
        </div>
      </div>

      <!-- Headers (full replacement) -->
      <div class="field" class:on={ovr?.headers != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.headers != null}
              onchange={(e) => patch("headers", (e.currentTarget as HTMLInputElement).checked ? [...request.headers] : null)}
            />
            <span class="dot"></span>
            <span class="lbl">Headers</span>
          </label>
          <span class="head-note">replaces the base list entirely</span>
        </div>
        {#if ovr?.headers != null}
          <KeyValueTable
            rows={ovr.headers}
            onChange={(rows) => patch("headers", rows)}
            keyPlaceholder="Header"
            valuePlaceholder="Value"
          />
        {/if}
        <div class="base">
          <span class="base-tag">base</span>
          <span class="base-summary">
            {#if request.headers.length === 0}none
            {:else}{request.headers.length} header{request.headers.length === 1 ? "" : "s"}
            {/if}
          </span>
        </div>
      </div>

      <!-- Params (full replacement) -->
      <div class="field" class:on={ovr?.params != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.params != null}
              onchange={(e) => patch("params", (e.currentTarget as HTMLInputElement).checked ? [...request.params] : null)}
            />
            <span class="dot"></span>
            <span class="lbl">Query params</span>
          </label>
          <span class="head-note">replaces the base list entirely</span>
        </div>
        {#if ovr?.params != null}
          <KeyValueTable
            rows={ovr.params}
            onChange={(rows) => patch("params", rows)}
            keyPlaceholder="Param"
            valuePlaceholder="Value"
          />
        {/if}
        <div class="base">
          <span class="base-tag">base</span>
          <span class="base-summary">
            {#if request.params.length === 0}none
            {:else}{request.params.length} param{request.params.length === 1 ? "" : "s"}
            {/if}
          </span>
        </div>
      </div>

      <!-- Body type + content -->
      <div class="field" class:on={ovr?.bodyType != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.bodyType != null}
              onchange={(e) => patch("bodyType", (e.currentTarget as HTMLInputElement).checked ? request.bodyType : null)}
            />
            <span class="dot"></span>
            <span class="lbl">Body type</span>
          </label>
        </div>
        {#if ovr?.bodyType != null}
          <Segment value={ovr.bodyType} options={BODY_TYPES} onChange={(t) => patch("bodyType", t)} />
        {/if}
        <div class="base"><span class="base-tag">base</span>{request.bodyType}</div>
      </div>

      <div class="field" class:on={ovr?.bodyContent != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.bodyContent != null}
              onchange={(e) => patch("bodyContent", (e.currentTarget as HTMLInputElement).checked ? request.bodyContent : null)}
            />
            <span class="dot"></span>
            <span class="lbl">Body content</span>
          </label>
        </div>
        {#if ovr?.bodyContent != null}
          <div class="body-editor">
            <CodeMirrorEditor
              value={ovr.bodyContent}
              onChange={(v) => patch("bodyContent", v)}
              language={(ovr.bodyType ?? request.bodyType) === "json" ? "json" : "plain"}
            />
          </div>
        {/if}
        <div class="base mono base-body" title={request.bodyContent}>
          <span class="base-tag">base</span>
          <span class="base-summary">
            {#if request.bodyContent.length === 0}empty
            {:else}{request.bodyContent.length} char{request.bodyContent.length === 1 ? "" : "s"}
            {/if}
          </span>
        </div>
      </div>

      <!-- Auth -->
      <div class="field" class:on={ovr?.auth != null}>
        <div class="field-head">
          <label class="toggle">
            <input
              type="checkbox"
              checked={ovr?.auth != null}
              onchange={(e) => patch("auth", (e.currentTarget as HTMLInputElement).checked ? { ...request.auth } : null)}
            />
            <span class="dot"></span>
            <span class="lbl">Auth</span>
          </label>
        </div>
        {#if ovr?.auth != null}
          <div class="auth">
            <Select
              value={ovr.auth.type}
              options={AUTH_TYPES}
              onChange={(t) => patch("auth", defaultForAuth(t))}
              ariaLabel="Auth type"
            >
              {#snippet triggerLabel(o)}
                <span>{o.label}</span>
              {/snippet}
              {#snippet optionLabel(o)}
                <span>{o.label}</span>
              {/snippet}
            </Select>
            {#if ovr.auth.type === "bearer"}
              <input
                class="ap-input mono auth-input"
                type="text"
                placeholder="token"
                value={ovr.auth.token}
                oninput={(e) => patch("auth", { type: "bearer", token: (e.currentTarget as HTMLInputElement).value })}
              />
            {:else if ovr.auth.type === "basic"}
              <input
                class="ap-input mono auth-input"
                type="text"
                placeholder="username"
                value={ovr.auth.username}
                oninput={(e) => patch("auth", { ...(ovr.auth as { type: 'basic'; username: string; password: string }), username: (e.currentTarget as HTMLInputElement).value })}
              />
              <input
                class="ap-input mono auth-input"
                type="text"
                placeholder="password"
                value={ovr.auth.password}
                oninput={(e) => patch("auth", { ...(ovr.auth as { type: 'basic'; username: string; password: string }), password: (e.currentTarget as HTMLInputElement).value })}
              />
            {:else if ovr.auth.type === "apikey"}
              <input
                class="ap-input mono auth-input"
                type="text"
                placeholder="name"
                value={ovr.auth.name}
                oninput={(e) => patch("auth", { ...(ovr.auth as { type: 'apikey'; name: string; value: string; in: 'header' | 'query' }), name: (e.currentTarget as HTMLInputElement).value })}
              />
              <input
                class="ap-input mono auth-input"
                type="text"
                placeholder="value"
                value={ovr.auth.value}
                oninput={(e) => patch("auth", { ...(ovr.auth as { type: 'apikey'; name: string; value: string; in: 'header' | 'query' }), value: (e.currentTarget as HTMLInputElement).value })}
              />
            {/if}
          </div>
        {/if}
        <div class="base"><span class="base-tag">base</span>{request.auth.type}</div>
      </div>
    </div>
  </div>
{/if}

<style>
  .root {
    flex: 1;
    overflow: auto;
    background: var(--bg);
    display: flex;
    flex-direction: column;
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 28px;
    color: var(--fg-muted);
    text-align: center;
  }
  .empty-title {
    color: var(--fg);
    font-size: 13.5px;
    font-weight: 600;
  }
  .empty-msg {
    font-size: 12px;
    color: var(--fg-muted);
    line-height: 1.55;
    max-width: 420px;
  }
  .empty code {
    background: var(--surface-2);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--accent);
  }
  .lock-glow {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: linear-gradient(
      180deg,
      rgba(245, 158, 11, 0.18),
      rgba(245, 158, 11, 0.06)
    );
    border: 1px solid rgba(245, 158, 11, 0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    margin-bottom: 4px;
  }

  .bar {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border-soft);
    background: var(--surface);
    font-size: 11.5px;
    color: var(--fg-muted);
    line-height: 1.5;
  }
  .bar :global(.bar-icon) {
    color: var(--accent);
    margin-top: 1px;
  }
  .bar-msg b {
    color: var(--fg);
    font-weight: 600;
  }
  .bar .mono {
    color: var(--fg-faint);
  }
  .grow {
    flex: 1;
  }

  .fields {
    display: flex;
    flex-direction: column;
  }
  .field {
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-soft);
    display: flex;
    flex-direction: column;
    gap: 8px;
    position: relative;
  }
  .field.on {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
  }
  .field-head {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
  }
  .toggle input {
    accent-color: var(--accent);
    width: 13px;
    height: 13px;
    cursor: pointer;
  }
  .toggle .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--fg-faint);
  }
  .field.on .toggle .dot {
    background: var(--accent);
    box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.12);
  }
  .toggle .lbl {
    font-weight: 600;
    color: var(--fg-dim);
  }
  .field.on .toggle .lbl {
    color: var(--accent);
  }
  .head-note {
    font-size: 10.5px;
    color: var(--fg-faint);
    font-style: italic;
  }

  .cm-shell {
    height: 30px;
    padding: 0 10px;
    background: var(--bg);
    border: 1px solid var(--accent-bd);
    border-radius: 6px;
    color: var(--fg);
    font: 12px/1 var(--mono);
    outline: none;
    box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.08);
  }
  .cm-shell:focus {
    border-color: var(--accent);
  }
  .cm-shell::placeholder {
    color: var(--fg-faint);
    font-style: italic;
  }

  .method-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .opt-label {
    color: var(--fg-dim);
    font-size: 11px;
  }

  .body-editor {
    height: 180px;
    border: 1px solid var(--accent-bd);
    border-radius: 6px;
    overflow: hidden;
  }

  .auth {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .auth-input {
    min-width: 160px;
    flex: 1;
  }

  .base {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--fg-faint);
    line-height: 1.4;
    word-break: break-all;
  }
  .base-tag {
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    background: var(--surface-2);
    color: var(--fg-faint);
    padding: 2px 6px;
    border-radius: 3px;
  }
  .base-summary {
    font-style: italic;
  }
  .base-body {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
