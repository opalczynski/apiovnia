<!--
  OnboardingOverlay — full-shell welcome panel rendered only on a fresh DB
  (zero projects, not still loading). Replaces the would-be "three empty
  panels" first impression with a single big CTA + a tour of what each
  panel does.

  Disappears the instant the first project is created — the rest of the
  app picks up from there (cascade auto-pick + DetailPanel's per-state
  CTAs handle the rest of the funnel).
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { app } from "$lib/stores/app.svelte";
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";

  async function newProject() {
    const name = await dialogs.prompt({
      title: "New project",
      message: "Projects group related collections (typically one per API).",
      placeholder: "e.g. UDL, my-api…",
      confirmLabel: "Create project",
    });
    if (name) await app.createProject(name);
  }

  // Optional shortcut: spin up the first project pre-populated from an
  // OpenAPI spec. We still need a project to import into, so we prompt
  // for a name first.
  async function importOpenapi() {
    const name = await dialogs.prompt({
      title: "Name the project",
      message: "We'll import the OpenAPI spec into a new collection inside this project.",
      placeholder: "e.g. petstore",
      confirmLabel: "Continue",
    });
    if (!name) return;
    const path = await openDialog({
      title: "Pick an OpenAPI YAML / JSON file",
      multiple: false,
      directory: false,
      filters: [{ name: "OpenAPI", extensions: ["yaml", "yml", "json"] }],
    });
    if (typeof path !== "string") return;
    await app.createProject(name);
    if (app.activeProjectId) {
      await app.importOpenapiForProject(app.activeProjectId, path);
    }
  }
</script>

<div class="overlay">
  <div class="card">
    <div class="brand">
      <div class="logo">A</div>
      <div>
        <div class="brand-name">Apiovnia</div>
        <div class="brand-sub">Local-first REST client</div>
      </div>
    </div>

    <h1>Welcome.</h1>
    <p class="lede">
      One SQLite file. No accounts, no telemetry, no cloud. Start with a project —
      collections and requests live inside.
    </p>

    <div class="actions">
      <button class="ap-btn cta primary" onclick={() => void newProject()}>
        <Icon d={IC.plus} />
        <span>Create your first project</span>
      </button>
      <button class="ap-btn secondary" onclick={() => void importOpenapi()}>
        <Icon d={IC.arrowR} />
        <span>Start from OpenAPI spec…</span>
      </button>
    </div>

    <ul class="tour">
      <li>
        <span class="step">1</span>
        <div>
          <strong>Left panel</strong> — projects & collections. Tree-style
          navigation; right-click for rename / delete / OpenAPI import.
        </div>
      </li>
      <li>
        <span class="step">2</span>
        <div>
          <strong>Middle panel</strong> — requests in the active collection.
          Filter by name or HTTP method.
        </div>
      </li>
      <li>
        <span class="step">3</span>
        <div>
          <strong>Right panel</strong> — the editor. URL, headers, body, auth,
          per-environment overrides, then Send.
        </div>
      </li>
    </ul>

    <footer>
      <span class="kbd-row">
        <span class="ap-kbd">⌘P</span> palette
        <span class="ap-kbd">⌘K</span> filter sidebar
        <span class="ap-kbd">⌘N</span> new request
        <span class="ap-kbd">⌘1</span>/<span class="ap-kbd">2</span>/<span class="ap-kbd">3</span>
        focus panel
        <span class="ap-kbd">⌘↵</span> send
      </span>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 32px 0 0 0; /* sits below the title bar */
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 700;
    overflow-y: auto;
  }
  .card {
    width: min(560px, 100%);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 28px 28px 22px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 18px;
  }
  .logo {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: linear-gradient(135deg, var(--accent), var(--accent-dim));
    color: var(--on-accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    font-size: 13px;
  }
  .brand-name {
    color: var(--fg);
    font-weight: 600;
    font-size: 13px;
    line-height: 1.1;
  }
  .brand-sub {
    color: var(--fg-muted);
    font-size: 10.5px;
    line-height: 1.2;
  }
  h1 {
    margin: 0 0 8px;
    font-size: 22px;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .lede {
    margin: 0 0 22px;
    color: var(--fg-dim);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 24px;
  }
  .ap-btn.cta.primary {
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
    font-weight: 600;
  }
  .ap-btn.cta.primary:hover {
    background: var(--accent-hi);
  }
  .ap-btn.secondary {
    background: transparent;
    color: var(--fg);
    border: 1px solid var(--border-strong);
  }
  .ap-btn.secondary:hover {
    background: var(--surface-2);
    border-color: var(--fg-muted);
  }
  .tour {
    list-style: none;
    padding: 0;
    margin: 0 0 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    border-top: 1px solid var(--border-soft);
    padding-top: 18px;
  }
  .tour li {
    display: grid;
    grid-template-columns: 22px 1fr;
    gap: 12px;
    align-items: start;
    color: var(--fg-dim);
    font-size: 12px;
    line-height: 1.5;
  }
  .tour strong {
    color: var(--fg);
    font-weight: 600;
  }
  .step {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--surface-2);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
    flex-shrink: 0;
  }
  footer {
    border-top: 1px solid var(--border-soft);
    padding-top: 12px;
    color: var(--fg-muted);
    font-size: 10.5px;
  }
  .kbd-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
</style>
