<!--
  UrlBar — method picker + URL input + Send button + env picker placeholder.

  Edits flow up via `onPatch`. The parent (DetailPanel) wires this to the
  store's `updateActiveRequest`, which debounces persistence. The env picker
  fires `onManageEnvs` when the user clicks "Manage envs…"; the parent
  owns the modal.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import MethodBadge from "$lib/components/MethodBadge.svelte";
  import Select, { type SelectOption } from "$lib/components/ui/Select.svelte";
  import EnvSelector from "$lib/components/request/EnvSelector.svelte";
  import { app } from "$lib/stores/app.svelte";
  import type { HttpMethod, Request } from "$lib/types/domain";

  type Props = {
    request: Request;
    onPatch: (patch: Partial<Request>) => void;
    onManageEnvs: () => void;
  };

  const { request, onPatch, onManageEnvs }: Props = $props();

  function send() {
    void app.executeActive();
  }
  function onUrlKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      send();
    }
  }

  const METHODS: SelectOption<HttpMethod>[] = [
    { value: "GET", label: "GET" },
    { value: "POST", label: "POST" },
    { value: "PUT", label: "PUT" },
    { value: "PATCH", label: "PATCH" },
    { value: "DELETE", label: "DELETE" },
    { value: "HEAD", label: "HEAD" },
    { value: "OPTIONS", label: "OPTIONS" },
  ];

  function setMethod(m: HttpMethod) {
    onPatch({ method: m });
  }
  function setUrl(e: Event) {
    onPatch({ url: (e.currentTarget as HTMLInputElement).value });
  }
</script>

<div class="urlbar">
  <Select
    value={request.method}
    options={METHODS}
    onChange={setMethod}
    ariaLabel="HTTP method"
    class="method-select"
  >
    {#snippet triggerLabel(o)}
      <MethodBadge method={o.value} />
    {/snippet}
    {#snippet optionLabel(o)}
      <MethodBadge method={o.value} />
      <span class="opt-label mono">{o.label}</span>
    {/snippet}
  </Select>

  <div class="url-shell">
    <input
      class="url mono"
      type="text"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      placeholder="https://api.example.com/path"
      value={request.url}
      oninput={setUrl}
      onkeydown={onUrlKeydown}
    />
    <span class="hint ap-kbd" title="Send (⌘↵)">⌘↵</span>
  </div>

  <button
    class="ap-btn cta send"
    onclick={send}
    disabled={app.executing || !request.url.trim()}
    title={request.url.trim() ? "Send (⌘↵)" : "Set a URL first"}
  >
    {#if app.executing}
      <span class="spin"></span>
      <span>Sending…</span>
    {:else}
      <Icon d={IC.send} />
      <span>Send</span>
    {/if}
  </button>

  <EnvSelector onManage={onManageEnvs} />
</div>

<style>
  .urlbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  :global(.method-select) {
    height: 30px;
    padding: 0 8px 0 6px;
  }
  .opt-label {
    color: var(--fg-dim);
    font-size: 11px;
  }
  .url-shell {
    flex: 1;
    display: flex;
    align-items: center;
    height: 30px;
    padding: 0 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    gap: 6px;
    transition: border-color 0.12s;
  }
  .url-shell:focus-within {
    border-color: var(--accent-bd);
    box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.08);
  }
  .url {
    flex: 1;
    height: 100%;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--fg);
    font-size: 12px;
    font-family: var(--mono);
    padding: 0;
  }
  .url::placeholder {
    color: var(--fg-faint);
  }
  .hint {
    flex-shrink: 0;
  }
  .send {
    height: 30px;
    padding: 0 16px;
  }
  .spin {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1.5px solid rgba(26, 17, 2, 0.3);
    border-top-color: rgba(26, 17, 2, 0.95);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
