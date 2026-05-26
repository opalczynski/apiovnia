<!--
  GraphQlEditor — the split editor for a `BodyType::GraphQl` request body.

  Two stacked CodeMirror panes: the GraphQL document on top, the operation
  variables (JSON) below. Both are encoded together into `bodyContent` as a
  `{query, variables}` JSON blob — the same single-column trick Form and
  Multipart use. The Rust executor wraps that into the `application/json`
  wire body `{"query": …, "variables": …}` at send time.

  Decoding is lenient (mirrors `GraphQlBody::parse` in apiovnia-core): an
  empty or stale `bodyContent` reads as a blank body, and the first keystroke
  rewrites it into a well-formed envelope.
-->
<script lang="ts">
  import CodeMirrorEditor from "$lib/components/ui/CodeMirrorEditor.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import type { GraphQlBody } from "$lib/types/domain";

  type Props = {
    bodyContent: string;
    onChange: (next: string) => void;
  };

  const { bodyContent, onChange }: Props = $props();

  const decoded = $derived.by<GraphQlBody>(() => {
    if (!bodyContent.trim()) return { query: "", variables: "" };
    try {
      const parsed = JSON.parse(bodyContent) as Partial<GraphQlBody>;
      return {
        query: typeof parsed.query === "string" ? parsed.query : "",
        variables: typeof parsed.variables === "string" ? parsed.variables : "",
      };
    } catch {
      return { query: "", variables: "" };
    }
  });

  /** JSON parse-error count from the variables editor. */
  let varsLintCount = $state(0);

  // `jsonParseLinter` treats an empty document as a syntax error, so we only
  // arm the linter once the pane has content — a variable-less query should
  // not show a phantom JSON error.
  const lintVariables = $derived(decoded.variables.trim().length > 0);

  function setQuery(query: string) {
    onChange(JSON.stringify({ query, variables: decoded.variables }));
  }
  function setVariables(variables: string) {
    onChange(JSON.stringify({ query: decoded.query, variables }));
  }
</script>

<div class="gql">
  <section class="pane query-pane">
    <header class="pane-head">
      <span class="pane-title">Query</span>
      <span class="pane-hint">query · mutation · subscription · fragment</span>
    </header>
    <div class="pane-body">
      <CodeMirrorEditor
        value={decoded.query}
        onChange={setQuery}
        language="graphql"
      />
    </div>
  </section>

  <section class="pane vars-pane">
    <header class="pane-head">
      <span class="pane-title">Variables</span>
      <span class="pane-hint">JSON object</span>
      <span class="grow"></span>
      {#if varsLintCount > 0}
        <span class="lint" role="status">
          <Icon d={IC.x} size={10} />
          {varsLintCount} JSON {varsLintCount === 1 ? "error" : "errors"}
        </span>
      {/if}
    </header>
    <div class="pane-body">
      <CodeMirrorEditor
        value={decoded.variables}
        onChange={setVariables}
        language="json"
        lint={lintVariables}
        onLintChange={(n) => (varsLintCount = n)}
      />
    </div>
  </section>
</div>

<style>
  .gql {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  /* Query takes the lion's share; variables get a fixed, generous footer. */
  .query-pane {
    flex: 1;
  }
  .vars-pane {
    flex: 0 0 38%;
    border-top: 1px solid var(--border-soft);
  }
  .pane-head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 5px 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border-soft);
    flex-shrink: 0;
  }
  .pane-title {
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--fg-dim);
  }
  .pane-hint {
    font-size: 10.5px;
    color: var(--fg-faint);
    font-style: italic;
  }
  .grow {
    flex: 1;
  }
  .lint {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    color: var(--err);
  }
  .pane-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
