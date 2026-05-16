<!--
  ResponseSent — "Request" tab. Shows what actually went on the wire:
  method, final URL (after any redirects reqwest applied), every header
  reqwest sent, and a syntax-highlighted preview of the body. The
  canonical debugging tool when an API rejects you and you need to
  confirm the auth / header / payload you thought you set actually made
  it out.
-->
<script lang="ts">
  import MethodBadge from "$lib/components/MethodBadge.svelte";
  import CodeMirrorEditor from "$lib/components/ui/CodeMirrorEditor.svelte";
  import type { ExecutionResult } from "$lib/types/domain";
  import HeadersGrid from "./HeadersGrid.svelte";
  import { contentTypeOf, langFromContentType } from "./format";

  type Props = { response: ExecutionResult };
  const { response }: Props = $props();
  const s = $derived(response.sent);

  const ct = $derived(contentTypeOf(s.headers));
  const bodyLang = $derived(langFromContentType(ct));

  /** Pretty-print JSON for the preview when possible (so the editor reads
   *  the same way the response Pretty view does). Falls back to raw. */
  const bodyDisplay = $derived.by(() => {
    if (bodyLang !== "json" || !s.bodyPreview) return s.bodyPreview;
    try {
      return JSON.stringify(JSON.parse(s.bodyPreview), null, 2);
    } catch {
      return s.bodyPreview;
    }
  });

  const previewTruncated = $derived(s.bodyPreview.length < s.bodySizeBytes);
</script>

<div class="wrap">
  <div class="top">
    <MethodBadge method={s.method} />
    <span class="url mono">{s.url}</span>
  </div>

  <section class="section">
    <header>
      Headers <span class="count">{s.headers.length}</span>
    </header>
    <HeadersGrid headers={s.headers} emptyMessage="No request headers." />
  </section>

  <section class="section body-section">
    <header>
      Body
      {#if s.bodySizeBytes > 0}
        <span class="count">{s.bodySizeBytes.toLocaleString()} B</span>
        {#if ct}
          <span class="count">{ct}</span>
        {/if}
        {#if previewTruncated}
          <span class="count warn">preview only</span>
        {/if}
      {/if}
    </header>
    {#if s.bodySizeBytes === 0}
      <div class="empty">No request body.</div>
    {:else}
      <div class="editor">
        <CodeMirrorEditor
          value={bodyDisplay}
          onChange={() => {
            /* read-only */
          }}
          language={bodyLang}
          readOnly
        />
      </div>
    {/if}
  </section>
</div>

<style>
  .wrap {
    flex: 1;
    overflow: auto;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 0;
  }
  .url {
    color: var(--fg);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .section header {
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-dim);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-top: 1px solid var(--border-soft);
    border-bottom: 1px solid var(--border-soft);
    background: var(--surface);
  }
  .section header .count {
    font-size: 10px;
    color: var(--fg-faint);
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
    background: var(--surface-2);
    padding: 1px 6px;
    border-radius: 3px;
  }
  .section header .count.warn {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 12%, transparent);
  }
  .empty {
    padding: 14px;
    color: var(--fg-faint);
    font-size: 11.5px;
    font-style: italic;
  }

  /* Body editor — fixed-ish height so the section stays scrollable rather
     than pushing the wrap into an extreme tall layout. */
  .body-section {
    display: flex;
    flex-direction: column;
  }
  .editor {
    min-height: 180px;
    max-height: 60vh;
    border-bottom: 1px solid var(--border-soft);
    display: flex;
    flex-direction: column;
  }
</style>
