<!--
  AuthTab — pick an auth type, then fill the corresponding fields.

  Auth state is encoded as the `AuthConfig` discriminated union from
  `apiovnia-core::model::AuthConfig`. Switching type clears unrelated fields.
-->
<script lang="ts">
  import Select, { type SelectOption } from "$lib/components/ui/Select.svelte";
  import type { AuthConfig, Request } from "$lib/types/domain";

  type Props = {
    request: Request;
    onPatch: (patch: Partial<Request>) => void;
  };

  const { request, onPatch }: Props = $props();

  type AuthKind = AuthConfig["type"];

  const KINDS: SelectOption<AuthKind>[] = [
    { value: "none", label: "No auth" },
    { value: "bearer", label: "Bearer token" },
    { value: "basic", label: "Basic auth" },
    { value: "apikey", label: "API key" },
  ];

  const API_KEY_LOCATIONS: SelectOption<"header" | "query">[] = [
    { value: "header", label: "Header" },
    { value: "query", label: "Query param" },
  ];

  function setKind(k: AuthKind) {
    const next: AuthConfig = (() => {
      switch (k) {
        case "none":
          return { type: "none" };
        case "bearer":
          return { type: "bearer", token: "" };
        case "basic":
          return { type: "basic", username: "", password: "" };
        case "apikey":
          return { type: "apikey", name: "", value: "", in: "header" };
      }
    })();
    onPatch({ auth: next });
  }

  function patchAuth(p: Partial<AuthConfig>) {
    onPatch({ auth: { ...request.auth, ...p } as AuthConfig });
  }
</script>

<div class="tab">
  <div class="row">
    <span class="lbl">Auth type</span>
    <Select
      value={request.auth.type}
      options={KINDS}
      onChange={setKind}
      ariaLabel="Auth type"
      matchAnchorWidth
    />
  </div>

  {#if request.auth.type === "none"}
    <div class="empty">
      <strong>No authentication.</strong>
      <span class="hint">
        Most public APIs don't need it. Pick Bearer / Basic / API key above
        to attach credentials per request.
      </span>
    </div>
  {:else if request.auth.type === "bearer"}
    {@const a = request.auth}
    <div class="row">
      <span class="lbl">Token</span>
      <input
        class="ap-input field mono"
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder="eyJhbGciOiJIUzI1NiIs…"
        value={a.token}
        oninput={(e) =>
          patchAuth({ token: (e.currentTarget as HTMLInputElement).value })}
      />
    </div>
    <p class="hint inline">
      Sent as <span class="mono">Authorization: Bearer …</span>
    </p>
  {:else if request.auth.type === "basic"}
    {@const a = request.auth}
    <div class="row">
      <span class="lbl">Username</span>
      <input
        class="ap-input field mono"
        type="text"
        autocomplete="off"
        value={a.username}
        oninput={(e) =>
          patchAuth({ username: (e.currentTarget as HTMLInputElement).value })}
      />
    </div>
    <div class="row">
      <span class="lbl">Password</span>
      <input
        class="ap-input field mono"
        type="password"
        autocomplete="off"
        value={a.password}
        oninput={(e) =>
          patchAuth({ password: (e.currentTarget as HTMLInputElement).value })}
      />
    </div>
    <p class="hint inline">
      Sent as <span class="mono">Authorization: Basic base64(user:pass)</span>
    </p>
  {:else if request.auth.type === "apikey"}
    {@const a = request.auth}
    <div class="row">
      <span class="lbl">Key name</span>
      <input
        class="ap-input field mono"
        type="text"
        autocomplete="off"
        placeholder="X-API-Key"
        value={a.name}
        oninput={(e) =>
          patchAuth({ name: (e.currentTarget as HTMLInputElement).value })}
      />
    </div>
    <div class="row">
      <span class="lbl">Value</span>
      <input
        class="ap-input field mono"
        type="text"
        autocomplete="off"
        value={a.value}
        oninput={(e) =>
          patchAuth({ value: (e.currentTarget as HTMLInputElement).value })}
      />
    </div>
    <div class="row">
      <span class="lbl">Send in</span>
      <Select
        value={a.in}
        options={API_KEY_LOCATIONS}
        onChange={(v) => patchAuth({ in: v })}
        ariaLabel="API key location"
        matchAnchorWidth
      />
    </div>
  {/if}
</div>

<style>
  .tab {
    flex: 1;
    overflow: auto;
    background: var(--bg);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 720px;
  }
  .row {
    display: grid;
    grid-template-columns: 120px 1fr;
    align-items: center;
    gap: 12px;
  }
  .lbl {
    font-size: 11px;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 600;
  }
  .field {
    width: 100%;
    height: 30px;
    font-size: 12.5px;
  }
  .hint.inline {
    margin: 0;
    padding-left: 132px;
    font-size: 11px;
    color: var(--fg-faint);
  }
  .empty {
    padding: 18px 0 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .empty strong {
    color: var(--fg);
    font-weight: 600;
    font-size: 13px;
  }
  .empty .hint {
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.5;
    max-width: 480px;
  }
</style>
