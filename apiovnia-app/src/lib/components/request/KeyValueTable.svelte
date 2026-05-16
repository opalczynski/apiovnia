<!--
  KeyValueTable — generic editable list of `{ key, value, enabled }` rows.
  Used for query params, headers, and form-encoded body fields.

  Behaviour:
    - One always-empty "draft" row at the bottom — typing anything in it
      promotes it to a real row.
    - Checkbox toggles `enabled`.
    - `×` deletes the row.
    - Changes flow up via `onChange(rows)`. Parent debounces persistence.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import type { KeyValue } from "$lib/types/domain";

  type Props = {
    rows: KeyValue[];
    onChange: (next: KeyValue[]) => void;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
  };

  const {
    rows,
    onChange,
    keyPlaceholder = "Key",
    valuePlaceholder = "Value",
  }: Props = $props();

  function setRow(idx: number, patch: Partial<KeyValue>) {
    const next = rows.map((r, i) => (i === idx ? { ...r, ...patch } : r));
    onChange(next);
  }

  function deleteRow(idx: number) {
    onChange(rows.filter((_, i) => i !== idx));
  }

  function draftChange(field: "key" | "value", v: string) {
    if (!v) return;
    const draft: KeyValue = {
      key: field === "key" ? v : "",
      value: field === "value" ? v : "",
      enabled: true,
    };
    onChange([...rows, draft]);
  }
</script>

<div class="kv">
  <div class="head">
    <span class="col-on"></span>
    <span class="col-k">{keyPlaceholder}</span>
    <span class="col-v">{valuePlaceholder}</span>
    <span class="col-x"></span>
  </div>

  {#each rows as r, i (i)}
    <div class="row" class:disabled={!r.enabled}>
      <label class="col-on" title={r.enabled ? "Disable" : "Enable"}>
        <input
          type="checkbox"
          checked={r.enabled}
          onchange={(e) =>
            setRow(i, { enabled: (e.currentTarget as HTMLInputElement).checked })}
        />
      </label>
      <input
        class="cell mono col-k"
        type="text"
        value={r.key}
        placeholder={keyPlaceholder}
        oninput={(e) =>
          setRow(i, { key: (e.currentTarget as HTMLInputElement).value })}
      />
      <input
        class="cell mono col-v"
        type="text"
        value={r.value}
        placeholder={valuePlaceholder}
        oninput={(e) =>
          setRow(i, { value: (e.currentTarget as HTMLInputElement).value })}
      />
      <button
        type="button"
        class="col-x kill"
        title="Delete row"
        onclick={() => deleteRow(i)}
        aria-label="Delete row"
      >
        <Icon d={IC.x} size={12} />
      </button>
    </div>
  {/each}

  <!-- Always-empty draft row. -->
  <div class="row draft">
    <span class="col-on"></span>
    <input
      class="cell mono col-k"
      type="text"
      value=""
      placeholder={keyPlaceholder}
      oninput={(e) => {
        const v = (e.currentTarget as HTMLInputElement).value;
        if (v) {
          draftChange("key", v);
          (e.currentTarget as HTMLInputElement).value = "";
        }
      }}
    />
    <input
      class="cell mono col-v"
      type="text"
      value=""
      placeholder={valuePlaceholder}
      oninput={(e) => {
        const v = (e.currentTarget as HTMLInputElement).value;
        if (v) {
          draftChange("value", v);
          (e.currentTarget as HTMLInputElement).value = "";
        }
      }}
    />
    <span class="col-x"></span>
  </div>
</div>

<style>
  .kv {
    display: flex;
    flex-direction: column;
    width: 100%;
    font-family: var(--ui);
  }
  .head {
    display: grid;
    grid-template-columns: 28px 1fr 1.4fr 28px;
    align-items: center;
    padding: 6px 8px;
    font: 600 10px/1 var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    border-bottom: 1px solid var(--border-soft);
  }
  .row {
    display: grid;
    grid-template-columns: 28px 1fr 1.4fr 28px;
    align-items: stretch;
    border-bottom: 1px solid var(--border-soft);
  }
  .row.disabled .cell {
    color: var(--fg-faint);
    text-decoration: line-through;
  }
  .row.draft .cell {
    color: var(--fg-faint);
    font-style: italic;
  }
  .col-on {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .col-on input[type="checkbox"] {
    accent-color: var(--accent);
    width: 13px;
    height: 13px;
    cursor: pointer;
  }
  .cell {
    height: 30px;
    padding: 0 8px;
    background: transparent;
    color: var(--fg);
    border: 0;
    font: 12px/1 var(--mono);
    outline: none;
  }
  .cell:focus {
    background: var(--surface-2);
  }
  .cell::placeholder {
    color: var(--fg-faint);
    font-style: italic;
  }
  .kill {
    width: 100%;
    height: 30px;
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .row:hover .kill {
    opacity: 1;
  }
  .kill:hover {
    color: var(--err);
  }
</style>
