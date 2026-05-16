<!--
  BodyTab — picks body type and renders the appropriate editor.
    - None: empty state explainer.
    - JSON / Raw: CodeMirror (JSON gets parse-lint).
    - Form: KeyValueTable serialised as `application/x-www-form-urlencoded`.
    - Multipart: per-row text-or-file picker, sent as `multipart/form-data`.

  Both Form and Multipart stash their rows as JSON inside `bodyContent` so
  the schema stays single-column — the Rust executor knows which decoder
  to use from `bodyType`.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import Segment from "$lib/components/ui/Segment.svelte";
  import CodeMirrorEditor from "$lib/components/ui/CodeMirrorEditor.svelte";
  import KeyValueTable from "../KeyValueTable.svelte";
  import MultipartTable from "../MultipartTable.svelte";
  import type {
    BodyType,
    KeyValue,
    MultipartField,
    Request,
  } from "$lib/types/domain";

  type Props = {
    request: Request;
    onPatch: (patch: Partial<Request>) => void;
  };

  const { request, onPatch }: Props = $props();

  /** JSON parse-error count from the editor. Drives the red banner. */
  let lintCount = $state(0);

  const TYPES = [
    { value: "none" as BodyType, label: "None" },
    { value: "json" as BodyType, label: "JSON" },
    { value: "form" as BodyType, label: "Form" },
    { value: "multipart" as BodyType, label: "Multipart" },
    { value: "raw" as BodyType, label: "Raw" },
  ];

  function setType(t: BodyType) {
    onPatch({ bodyType: t });
  }
  function setText(next: string) {
    onPatch({ bodyContent: next });
  }

  // Form body lives in `bodyContent` as a JSON-encoded KeyValue[] so we don't
  // need a second schema field. Empty/invalid → empty list.
  let formRows = $derived.by<KeyValue[]>(() => {
    if (request.bodyType !== "form") return [];
    if (!request.bodyContent) return [];
    try {
      const parsed = JSON.parse(request.bodyContent);
      return Array.isArray(parsed) ? (parsed as KeyValue[]) : [];
    } catch {
      return [];
    }
  });

  function setForm(next: KeyValue[]) {
    onPatch({ bodyContent: JSON.stringify(next) });
  }

  // Multipart rows — same trick, decoded by serde on the Rust side.
  let multipartRows = $derived.by<MultipartField[]>(() => {
    if (request.bodyType !== "multipart") return [];
    if (!request.bodyContent) return [];
    try {
      const parsed = JSON.parse(request.bodyContent);
      return Array.isArray(parsed) ? (parsed as MultipartField[]) : [];
    } catch {
      return [];
    }
  });

  function setMultipart(next: MultipartField[]) {
    onPatch({ bodyContent: JSON.stringify(next) });
  }

  function beautify() {
    if (request.bodyType !== "json") return;
    try {
      const pretty = JSON.stringify(JSON.parse(request.bodyContent), null, 2);
      onPatch({ bodyContent: pretty });
    } catch {
      // Silently no-op on invalid JSON — Phase 4 will surface lint errors.
    }
  }
</script>

<div class="tab">
  <div class="toolbar">
    <Segment
      value={request.bodyType}
      options={TYPES}
      onChange={setType}
      ariaLabel="Body type"
    />
    <span class="grow"></span>
    {#if request.bodyType === "json"}
      <button class="ap-btn sm ghost" onclick={beautify}>Beautify</button>
    {/if}
  </div>

  <div class="canvas">
    {#if request.bodyType === "none"}
      <div class="empty">
        <strong>No request body.</strong>
        <span class="hint">
          Pick a type above to attach JSON, a form-encoded payload, or raw text.
        </span>
      </div>
    {:else if request.bodyType === "json"}
      {#if lintCount > 0}
        <div class="lint-banner" role="status">
          <span class="lint-dot"></span>
          <Icon d={IC.x} size={11} />
          <span class="lint-msg">
            <strong>{lintCount}</strong>
            JSON parse {lintCount === 1 ? "error" : "errors"} — hover the
            indicator in the gutter for details.
          </span>
        </div>
      {/if}
      <CodeMirrorEditor
        value={request.bodyContent}
        onChange={setText}
        language="json"
        lint
        onLintChange={(n) => (lintCount = n)}
      />
    {:else if request.bodyType === "raw"}
      <CodeMirrorEditor
        value={request.bodyContent}
        onChange={setText}
        language="plain"
      />
    {:else if request.bodyType === "form"}
      <div class="form">
        <KeyValueTable
          rows={formRows}
          onChange={setForm}
          keyPlaceholder="Field"
          valuePlaceholder="Value"
        />
      </div>
    {:else if request.bodyType === "multipart"}
      <div class="form">
        <MultipartTable rows={multipartRows} onChange={setMultipart} />
      </div>
    {/if}
  </div>
</div>

<style>
  .tab {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-soft);
  }
  .grow {
    flex: 1;
  }
  .canvas {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--fg-muted);
    padding: 24px;
    text-align: center;
  }
  .empty strong {
    color: var(--fg);
    font-weight: 600;
    font-size: 13px;
  }
  .empty .hint {
    font-size: 11.5px;
    color: var(--fg-faint);
  }
  .form {
    flex: 1;
    overflow: auto;
  }

  /* JSON lint banner — sits above the editor in the body area so it never
     pushes content off-screen. Subtle red so it reads as "you should know"
     rather than "everything is broken". */
  .lint-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: color-mix(in srgb, var(--err) 10%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--err) 35%, transparent);
    color: var(--err);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .lint-banner :global(svg) {
    flex-shrink: 0;
  }
  .lint-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--err);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--err) 22%, transparent);
    flex-shrink: 0;
    animation: pulse-err 1.4s ease-in-out infinite;
  }
  .lint-msg strong {
    font-weight: 600;
    color: var(--err);
  }
  @keyframes pulse-err {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.55;
    }
  }
</style>
