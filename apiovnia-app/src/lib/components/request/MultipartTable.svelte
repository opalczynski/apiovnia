<!--
  MultipartTable — KeyValueTable's cousin for `multipart/form-data` rows.
  Each row picks `text` or `file`; file rows show the basename + size + a
  picker. The selected absolute path stays on disk and is read each time
  the request runs (so editing the file out-of-band picks up the change).
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { MultipartField } from "$lib/types/domain";

  type Props = {
    rows: MultipartField[];
    onChange: (next: MultipartField[]) => void;
  };

  const { rows, onChange }: Props = $props();

  const emptyRow: () => MultipartField = () => ({
    key: "",
    value: "",
    kind: "text",
    filePath: "",
    contentType: "",
    enabled: true,
  });

  function setRow(idx: number, patch: Partial<MultipartField>) {
    onChange(rows.map((r, i) => (i === idx ? { ...r, ...patch } : r)));
  }
  function deleteRow(idx: number) {
    onChange(rows.filter((_, i) => i !== idx));
  }
  function appendRow(seed: Partial<MultipartField>) {
    onChange([...rows, { ...emptyRow(), ...seed }]);
  }

  function toggleKind(idx: number) {
    const r = rows[idx];
    setRow(idx, {
      kind: r.kind === "text" ? "file" : "text",
      // Clear the unused side so accidentally-typed values don't survive a flip.
      ...(r.kind === "text"
        ? { value: "" }
        : { filePath: "", contentType: "" }),
    });
  }

  async function pickFile(idx: number) {
    const selected = await open({ multiple: false, directory: false });
    if (typeof selected === "string") {
      setRow(idx, { filePath: selected });
    }
  }

  async function pickFileForDraft() {
    const selected = await open({ multiple: false, directory: false });
    if (typeof selected === "string") {
      appendRow({ kind: "file", filePath: selected });
    }
  }

  /** Pull the last path segment so the table shows `report.pdf` instead of
   *  `/Users/sebastian/.../report.pdf`. Full path stays in the title attr. */
  function basename(p: string): string {
    if (!p) return "";
    const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return i === -1 ? p : p.slice(i + 1);
  }
</script>

<div class="mp">
  <div class="head">
    <span class="col-on"></span>
    <span class="col-k">Field</span>
    <span class="col-kind">Kind</span>
    <span class="col-v">Value / file</span>
    <span class="col-x"></span>
  </div>

  {#each rows as r, i (i)}
    <div class="row" class:disabled={!r.enabled}>
      <label class="col-on" title={r.enabled ? "Disable" : "Enable"}>
        <input
          type="checkbox"
          checked={r.enabled}
          onchange={(e) =>
            setRow(i, {
              enabled: (e.currentTarget as HTMLInputElement).checked,
            })}
        />
      </label>

      <input
        class="cell mono col-k"
        type="text"
        value={r.key}
        placeholder="field"
        oninput={(e) =>
          setRow(i, { key: (e.currentTarget as HTMLInputElement).value })}
      />

      <button
        type="button"
        class="kind-toggle col-kind"
        class:file={r.kind === "file"}
        title={r.kind === "text"
          ? "Switch to file part"
          : "Switch to text part"}
        onclick={() => toggleKind(i)}
      >
        {r.kind}
      </button>

      {#if r.kind === "text"}
        <input
          class="cell mono col-v"
          type="text"
          value={r.value}
          placeholder="value"
          oninput={(e) =>
            setRow(i, { value: (e.currentTarget as HTMLInputElement).value })}
        />
      {:else}
        <div class="file-cell col-v">
          {#if r.filePath}
            <span class="file-name mono" title={r.filePath}>
              {basename(r.filePath)}
            </span>
            <button
              type="button"
              class="ap-btn sm ghost"
              title="Pick a different file"
              onclick={() => void pickFile(i)}
            >
              Change
            </button>
            <input
              class="cell mono ct-input"
              type="text"
              value={r.contentType}
              placeholder="content-type (auto)"
              oninput={(e) =>
                setRow(i, {
                  contentType: (e.currentTarget as HTMLInputElement).value,
                })}
              title="Override the MIME type. Leave empty to auto-detect from the file extension."
            />
          {:else}
            <button
              type="button"
              class="ap-btn sm"
              onclick={() => void pickFile(i)}
            >
              <Icon d={IC.folder} size={12} /><span>Choose file…</span>
            </button>
          {/if}
        </div>
      {/if}

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

  <!-- Draft row: key + value/file. Typing in the key promotes to a real text row;
       clicking "Add file" promotes to a real file row. -->
  <div class="row draft">
    <span class="col-on"></span>
    <input
      class="cell mono col-k"
      type="text"
      value=""
      placeholder="field"
      oninput={(e) => {
        const v = (e.currentTarget as HTMLInputElement).value;
        if (v) {
          appendRow({ key: v });
          (e.currentTarget as HTMLInputElement).value = "";
        }
      }}
    />
    <span class="col-kind draft-kind">text / file</span>
    <div class="col-v draft-v">
      <input
        class="cell mono draft-value"
        type="text"
        value=""
        placeholder="value"
        oninput={(e) => {
          const v = (e.currentTarget as HTMLInputElement).value;
          if (v) {
            appendRow({ value: v });
            (e.currentTarget as HTMLInputElement).value = "";
          }
        }}
      />
      <button
        type="button"
        class="ap-btn sm ghost"
        title="Pick a file to add as a multipart part"
        onclick={() => void pickFileForDraft()}
      >
        <Icon d={IC.folder} size={12} /><span>+ File</span>
      </button>
    </div>
    <span class="col-x"></span>
  </div>
</div>

<style>
  .mp {
    display: flex;
    flex-direction: column;
    width: 100%;
    font-family: var(--ui);
  }
  .head,
  .row {
    display: grid;
    grid-template-columns: 28px 1fr 70px 2fr 28px;
    align-items: stretch;
    border-bottom: 1px solid var(--border-soft);
  }
  .head {
    padding: 6px 8px;
    font: 600 10px/1 var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
  }
  .row.disabled .cell,
  .row.disabled .file-name,
  .row.disabled .kind-toggle {
    color: var(--fg-faint);
    text-decoration: line-through;
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

  .kind-toggle {
    margin: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--fg-dim);
    border-radius: 4px;
    font: 600 10px/1 var(--mono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    height: 22px;
  }
  .kind-toggle:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .kind-toggle.file {
    background: var(--accent-bg);
    border-color: var(--accent-bd);
    color: var(--accent);
  }

  .file-cell {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    min-width: 0;
  }
  .file-name {
    color: var(--fg);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 0 1 auto;
  }
  .ct-input {
    flex: 1;
    min-width: 100px;
    background: var(--bg);
    border: 1px solid var(--border-soft);
    border-radius: 4px;
    height: 22px;
    padding: 0 6px;
    font-size: 10.5px;
    color: var(--fg-muted);
  }
  .ct-input:focus {
    border-color: var(--accent-bd);
    color: var(--fg);
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

  .row.draft .cell {
    color: var(--fg-faint);
    font-style: italic;
  }
  .row.draft .cell:focus {
    color: var(--fg);
    font-style: normal;
  }
  .draft-kind {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font: 500 10px/1 var(--mono);
    color: var(--fg-faint);
  }
  .draft-v {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 8px;
  }
  .draft-value {
    flex: 1;
    min-width: 0;
  }
</style>
