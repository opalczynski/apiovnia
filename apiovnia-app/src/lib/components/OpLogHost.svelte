<!--
  OpLogHost — persistent bottom-right panel for OpenAPI import/export
  feedback. Tabular per-request row list + warnings section + "Download
  log" button to save the full text trail. Unlike `ToastHost` this does
  *not* auto-dismiss — closed only via the × button or "Done".

  Wider than a toast, narrower than a modal — feels like a Linear/Notion
  side-toast. The user can keep editing while it's up.
-->
<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";

  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import * as ipc from "$lib/api/ipc";
  import { app } from "$lib/stores/app.svelte";

  let downloading = $state(false);

  async function downloadLog() {
    const log = app.opLog;
    if (!log || downloading) return;
    downloading = true;
    try {
      const path = await save({
        defaultPath: log.logFilename,
        filters: [{ name: "Log", extensions: ["log", "txt"] }],
      });
      if (path) {
        await ipc.saveTextFile(path, log.logText);
        app.showToast("Log saved");
      }
    } catch (e) {
      app.showToast(
        `Couldn't save log: ${e instanceof Error ? e.message : String(e)}`,
        "err",
      );
    } finally {
      downloading = false;
    }
  }
</script>

{#if app.opLog}
  {@const log = app.opLog}
  <aside class="card" class:export={log.kind === "export"} role="status" aria-live="polite">
    <header class="head">
      <span class="head-icon" class:export={log.kind === "export"}>
        <Icon d={log.kind === "import" ? IC.arrowR : IC.send} size={13} />
      </span>
      <div class="head-text">
        <div class="title">{log.title}</div>
        <div class="subtitle">{log.subtitle}</div>
      </div>
      <button
        type="button"
        class="ap-btn icon sm ghost close-x"
        title="Close"
        onclick={() => app.dismissOpLog()}
      >
        <Icon d={IC.x} />
      </button>
    </header>

    <div class="body">
      {#if log.rows.length > 0}
        <div class="rows">
          {#each log.rows as r (r.name + r.path + r.method)}
            <div class="row">
              <span class="row-method m-{r.method}">{r.method}</span>
              <span class="row-path mono" title={r.path}>{r.path}</span>
              <span class="row-name" title={r.name}>{r.name}</span>
              {#if log.kind === "export" && (r.redactions ?? 0) > 0}
                <span class="row-red" title="{r.redactions} secret(s) stripped">
                  −{r.redactions}
                </span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if log.warnings.length > 0}
        <details class="warns" open>
          <summary>
            <span class="warn-icon"><Icon d={IC.filter} size={11} /></span>
            <span>Warnings ({log.warnings.length})</span>
          </summary>
          <ul>
            {#each log.warnings as w (w)}
              <li>{w}</li>
            {/each}
          </ul>
        </details>
      {/if}
    </div>

    <footer class="foot">
      <button
        type="button"
        class="ap-btn sm"
        onclick={() => void downloadLog()}
        disabled={downloading}
      >
        <Icon d={IC.copy} size={11} />
        <span>{downloading ? "Saving…" : "Download log"}</span>
      </button>
      <span class="grow"></span>
      <button type="button" class="ap-btn sm ghost" onclick={() => app.dismissOpLog()}>
        Done
      </button>
    </footer>
  </aside>
{/if}

<style>
  .card {
    position: fixed;
    right: 14px;
    bottom: 14px;
    width: 460px;
    max-width: calc(100vw - 28px);
    max-height: 65vh;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow:
      0 16px 40px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(245, 158, 11, 0.06);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--ui);
    z-index: 900;
    animation: oplog-in 180ms ease-out both;
  }
  @keyframes oplog-in {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--border-soft);
  }
  .head-icon {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--ok);
    background: color-mix(in srgb, var(--ok) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--ok) 30%, transparent);
    flex-shrink: 0;
  }
  .head-icon.export {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .head-text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subtitle {
    margin-top: 3px;
    font-size: 11px;
    color: var(--fg-muted);
  }
  .close-x {
    margin-top: -2px;
    margin-right: -4px;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 6px 6px 8px;
  }
  .rows {
    display: flex;
    flex-direction: column;
  }
  .row {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) minmax(0, 0.9fr) auto;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    font-size: 11.5px;
    align-items: center;
  }
  .row:hover {
    background: var(--hover);
  }
  .row-method {
    text-align: right;
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--fg-muted);
  }
  .row-method.m-GET { color: var(--m-get, var(--ok)); }
  .row-method.m-POST { color: var(--m-post, var(--accent)); }
  .row-method.m-PUT { color: var(--m-put, var(--warn)); }
  .row-method.m-PATCH { color: var(--m-patch, var(--warn)); }
  .row-method.m-DELETE { color: var(--m-delete, var(--err)); }
  .row-path {
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-name {
    color: var(--fg-faint);
    font-style: italic;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-red {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--accent);
    background: var(--accent-bg);
    border: 1px solid var(--accent-bd);
    border-radius: 3px;
    padding: 0 5px;
  }

  .warns {
    margin: 8px 6px 4px;
    border-top: 1px solid var(--border-soft);
    padding-top: 8px;
    font-size: 11px;
  }
  .warns summary {
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--warn);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-size: 10px;
  }
  .warn-icon {
    display: inline-flex;
    color: var(--warn);
  }
  .warns ul {
    margin: 6px 0 0;
    padding-left: 22px;
    color: var(--fg-muted);
    line-height: 1.5;
  }
  .warns li {
    margin: 2px 0;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-2);
  }
  .foot .ap-btn {
    height: 28px;
    padding: 0 12px;
    font-size: 11.5px;
  }
  .grow {
    flex: 1;
  }
</style>
