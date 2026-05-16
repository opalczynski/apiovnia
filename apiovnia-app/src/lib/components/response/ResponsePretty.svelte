<!--
  ResponsePretty — formatted response body. Picks the renderer from the
  response content-type:
    - application/json or *+json → JsonView (collapsible, ⌘F search,
      hover-copy). Falls back to read-only CodeMirror if the body fails
      to parse as JSON, so malformed payloads stay readable byte-for-byte.
    - text/html → CodeMirror (HTML grammar)
    - application/xml, text/xml, *+xml → CodeMirror (XML grammar)
    - text/* (plain, csv, …) → CodeMirror (plain)
    - binary / empty → friendly note instead of dumping bytes
-->
<script lang="ts">
  import CodeMirrorEditor from "$lib/components/ui/CodeMirrorEditor.svelte";
  import JsonView from "./JsonView.svelte";
  import type { ExecutionResult } from "$lib/types/domain";
  import { langFromContentType } from "./format";

  type Props = { response: ExecutionResult };
  const { response }: Props = $props();

  const lang = $derived(langFromContentType(response.contentType));

  /** Try to parse the body as JSON when we believe it is JSON. Null means
   *  "couldn't parse" — we fall back to raw CodeMirror so the user can
   *  still inspect a malformed payload. */
  const parsed = $derived.by(() => {
    if (lang !== "json" || !response.body) return null;
    try {
      return { ok: true as const, value: JSON.parse(response.body) as unknown };
    } catch {
      return { ok: false as const };
    }
  });

  /** For non-JSON or JSON that failed to parse: text to render in CM. */
  const cmDisplay = $derived(response.body);
</script>

<div class="wrap">
  {#if response.bodyKind === "empty"}
    <div class="empty">Empty response body.</div>
  {:else if response.bodyKind === "binarybase64"}
    <div class="empty">
      Binary payload ({response.contentType ?? "unknown type"}). Switch to
      <strong>Raw</strong> for the base64 string, or copy it from the toolbar.
    </div>
  {:else if parsed && parsed.ok}
    <JsonView data={parsed.value} />
  {:else}
    {#if lang === "json"}
      <div class="note">
        Could not parse as JSON — showing raw bytes. See the
        <strong>Raw</strong> tab for the unmodified body.
      </div>
    {/if}
    <div class="editor">
      <CodeMirrorEditor
        value={cmDisplay}
        onChange={() => {
          /* read-only */
        }}
        language={lang}
        readOnly
      />
    </div>
  {/if}
</div>

<style>
  .wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
  }
  .editor {
    flex: 1;
    min-height: 0;
  }
  .empty {
    padding: 24px;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.5;
    max-width: 560px;
  }
  .empty strong {
    color: var(--fg);
  }
  .note {
    padding: 6px 12px;
    border-bottom: 1px solid var(--border-soft);
    font-size: 10.5px;
    color: var(--warn);
    font-style: italic;
  }
  .note strong {
    color: var(--fg);
    font-style: normal;
  }
</style>
