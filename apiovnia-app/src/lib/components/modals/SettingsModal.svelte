<!--
  SettingsModal — per-user preferences (Phase 11).

  Left-rail section nav + right pane, mirroring EnvManageModal. Mounted
  behind `settings.open` in App.svelte. Every control writes straight to
  the `settings` store, which persists to localStorage and (for the theme)
  re-themes the whole shell live behind the modal.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { settings, type ThemeId, type HistoryLimit } from "$lib/stores/settings.svelte";

  type Props = { onClose: () => void };
  let { onClose }: Props = $props();

  type Section = "appearance" | "history" | "about";
  let section = $state<Section>("appearance");

  let dialogEl = $state<HTMLDialogElement>();
  $effect(() => {
    if (dialogEl && !dialogEl.open) dialogEl.showModal();
  });

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onClose();
  }

  const NAV: { id: Section; label: string; icon: string }[] = [
    { id: "appearance", label: "Appearance", icon: IC.settings },
    { id: "history", label: "History", icon: IC.history },
    { id: "about", label: "About", icon: IC.globe },
  ];

  function pickTheme(id: ThemeId) {
    settings.theme = id;
  }
</script>

<dialog
  bind:this={dialogEl}
  class="modal"
  oncancel={(e) => {
    e.preventDefault();
    onClose();
  }}
  onclick={onBackdropClick}
>
  <div class="card">
    <header class="head">
      <div class="title">Settings</div>
      <div class="msg">Preferences are stored locally — no account, no sync.</div>
      <button class="ap-btn icon sm ghost close-x" title="Close (Esc)" onclick={onClose}>
        <Icon d={IC.x} />
      </button>
    </header>

    <div class="body">
      <!-- Left rail: section nav -->
      <aside class="rail">
        {#each NAV as n (n.id)}
          <button
            class="nav-row"
            class:active={section === n.id}
            onclick={() => (section = n.id)}
          >
            <Icon d={n.icon} size={14} />
            <span>{n.label}</span>
          </button>
        {/each}
      </aside>

      <!-- Right pane -->
      <section class="pane">
        {#if section === "appearance"}
          <div class="sec-head">
            <div class="sec-title">Theme</div>
            <div class="sec-sub">
              Switches the whole interface instantly. The colour set drives
              the JSON viewer and code editor too.
            </div>
          </div>
          <div class="theme-grid">
            {#each settings.readonly.THEMES as t (t.id)}
              {@const selected = settings.theme === t.id}
              <button
                class="theme-card"
                class:selected
                onclick={() => pickTheme(t.id)}
                aria-pressed={selected}
              >
                <div class="theme-preview" style:background={t.swatch[0]}>
                  <div class="tp-rail" style:background={t.swatch[1]}></div>
                  <div class="tp-main">
                    <span class="tp-dot" style:background={t.swatch[2]}></span>
                    <span class="tp-line" style:background={t.swatch[1]}></span>
                    <span
                      class="tp-line short"
                      style:background={t.swatch[1]}
                    ></span>
                  </div>
                </div>
                <div class="theme-meta">
                  <div class="theme-name">
                    <span>{t.label}</span>
                    {#if selected}
                      <span class="tick"><Icon d={IC.check} size={12} /></span>
                    {/if}
                  </div>
                  <div class="theme-blurb">{t.blurb}</div>
                </div>
              </button>
            {/each}
          </div>
        {:else if section === "history"}
          <div class="sec-head">
            <div class="sec-title">Retention</div>
            <div class="sec-sub">
              How many recent executions the History panel loads. Older runs
              stay in the database either way — this only caps what's shown.
            </div>
          </div>
          <div class="field">
            <span class="field-label">Entries to load</span>
            <div class="segmented">
              {#each settings.readonly.HISTORY_LIMITS as n (n)}
                <button
                  class="seg"
                  class:on={settings.historyLimit === n}
                  onclick={() => (settings.historyLimit = n as HistoryLimit)}
                >
                  {n}
                </button>
              {/each}
            </div>
          </div>
          <div class="note">
            Takes effect the next time the History panel opens.
          </div>
        {:else}
          <div class="sec-head">
            <div class="sec-title">About</div>
          </div>
          <div class="about">
            <div class="about-name">Apiovnia</div>
            <div class="about-ver mono">v{__APP_VERSION__}</div>
            <p class="about-line">
              A local-first REST API client for solo devs — keyboard-first,
              SQLite-backed, no sync and no telemetry.
            </p>
            <dl class="about-grid">
              <dt>Identifier</dt>
              <dd class="mono">tech.trurl.apiovnia</dd>
              <dt>Storage</dt>
              <dd>Single local SQLite database</dd>
              <dt>Built with</dt>
              <dd>Tauri 2 · Rust · Svelte 5</dd>
              <dt>License</dt>
              <dd>MIT (provisional)</dd>
            </dl>
          </div>
        {/if}
      </section>
    </div>

    <footer class="foot">
      <span class="hint">Settings persist immediately to this machine.</span>
      <span class="grow"></span>
      <button class="ap-btn" onclick={onClose}>Done <span class="ap-kbd">esc</span></button>
    </footer>
  </div>
</dialog>

<style>
  .modal {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    max-width: 100vw;
    max-height: 100vh;
    margin: 0;
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--fg);
    display: grid;
    place-items: center;
  }
  .modal:not([open]) {
    display: none;
  }
  .modal::backdrop {
    background: radial-gradient(
      circle at 50% 40%,
      rgba(0, 0, 0, 0.55),
      rgba(0, 0, 0, 0.85) 70%
    );
    backdrop-filter: blur(2px);
  }

  .card {
    width: 720px;
    max-width: calc(100vw - 32px);
    height: 540px;
    max-height: calc(100vh - 32px);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-family: var(--ui);
  }

  .head {
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border-soft);
    position: relative;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--fg);
  }
  .msg {
    margin-top: 4px;
    font-size: 12px;
    color: var(--fg-muted);
  }
  .close-x {
    position: absolute;
    top: 12px;
    right: 12px;
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .rail {
    width: 168px;
    border-right: 1px solid var(--border-soft);
    background: var(--bg);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 9px;
    border: 0;
    background: transparent;
    border-radius: 6px;
    color: var(--fg-dim);
    font: 500 12px/1 var(--ui);
    cursor: pointer;
    text-align: left;
  }
  .nav-row:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .nav-row.active {
    background: var(--selected);
    color: var(--fg);
  }

  .pane {
    flex: 1;
    min-width: 0;
    overflow: auto;
    background: var(--bg);
    padding: 16px 18px;
  }

  .sec-head {
    margin-bottom: 14px;
  }
  .sec-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--fg);
  }
  .sec-sub {
    margin-top: 4px;
    font-size: 11.5px;
    color: var(--fg-muted);
    line-height: 1.5;
    max-width: 460px;
  }

  /* --- Theme picker --- */
  .theme-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .theme-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition:
      border-color 0.12s,
      background 0.12s;
  }
  .theme-card:hover {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  .theme-card.selected {
    border-color: var(--accent);
    background: var(--accent-bg);
  }
  .theme-preview {
    height: 58px;
    border-radius: 5px;
    border: 1px solid var(--border-soft);
    display: flex;
    overflow: hidden;
  }
  .tp-rail {
    width: 26%;
    height: 100%;
  }
  .tp-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    padding: 0 10px;
  }
  .tp-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
  }
  .tp-line {
    height: 5px;
    border-radius: 3px;
    width: 80%;
  }
  .tp-line.short {
    width: 52%;
  }
  .theme-meta {
    padding: 0 2px 2px;
  }
  .theme-name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--fg);
  }
  .tick {
    display: inline-flex;
    color: var(--accent);
  }
  .theme-blurb {
    margin-top: 2px;
    font-size: 10.5px;
    color: var(--fg-muted);
    line-height: 1.45;
  }

  /* --- History --- */
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 0;
  }
  .field-label {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .segmented {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .seg {
    padding: 5px 12px;
    border: 0;
    border-right: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg-muted);
    font: 500 11.5px/1 var(--mono);
    cursor: pointer;
  }
  .seg:last-child {
    border-right: 0;
  }
  .seg:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .seg.on {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .note {
    margin-top: 6px;
    font-size: 10.5px;
    color: var(--fg-faint);
    font-style: italic;
  }

  /* --- About --- */
  .about-name {
    font-size: 16px;
    font-weight: 700;
    color: var(--fg);
  }
  .about-ver {
    font-size: 11.5px;
    color: var(--accent);
    margin-top: 2px;
  }
  .about-line {
    margin: 10px 0 14px;
    font-size: 12px;
    color: var(--fg-dim);
    line-height: 1.55;
    max-width: 440px;
  }
  .about-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    margin: 0;
    font-size: 11.5px;
  }
  .about-grid dt {
    color: var(--fg-muted);
  }
  .about-grid dd {
    margin: 0;
    color: var(--fg-dim);
  }

  .foot {
    padding: 10px 16px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-2);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hint {
    font-size: 10.5px;
    color: var(--fg-faint);
  }
  .grow {
    flex: 1;
  }
  .foot .ap-btn {
    height: 28px;
    padding: 0 14px;
    font-size: 12px;
  }
</style>
