<!--
  UnlockEnvModal — prompts for the master password of a sealed env, then
  loads its session key. Closes on success and fires an optional `retry`
  callback (set by whoever opened the modal — typically the Send flow that
  bounced off `ENV_LOCKED`).

  Layout mirrors `design_artifacts/artifact-unlock.jsx`: lock-icon header,
  env summary card, password field with show/hide, Cancel + Unlock buttons.
  We skip the artifact's blurred-shell backdrop — the real shell is right
  there and the radial dim is enough context.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { app } from "$lib/stores/app.svelte";
  import type { EnvironmentId } from "$lib/types/domain";

  type Props = {
    envId: EnvironmentId;
    /** Optional callback fired on successful unlock — see store's `unlockPrompt`. */
    retry?: () => void | Promise<void>;
    onClose: () => void;
  };

  const { envId, retry, onClose }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let password = $state("");
  let show = $state(false);
  let working = $state(false);
  let errMsg = $state<string | null>(null);

  const env = $derived(app.envs.find((e) => e.id === envId) ?? null);

  $effect(() => {
    if (!dialogEl) return;
    dialogEl.showModal();
    queueMicrotask(() => inputEl?.focus());
  });

  async function submit() {
    if (!password || working) return;
    working = true;
    errMsg = null;
    try {
      await app.unlockEnv(envId, password);
      working = false;
      onClose();
      if (retry) await retry();
    } catch (e) {
      working = false;
      errMsg =
        e instanceof Error ? prettyErr(e.message) : "Unlock failed. Try again.";
      queueMicrotask(() => inputEl?.select());
    }
  }

  /** Turn the raw Rust error string into a one-liner the user can act on. */
  function prettyErr(msg: string): string {
    if (msg.includes("wrong password")) return "Wrong password — try again.";
    if (msg.startsWith("crypto:")) return msg.slice("crypto:".length).trim();
    return msg;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void submit();
    }
    // Esc handled by <dialog> oncancel.
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onClose();
  }

  // Pure helpers — kept inside the instance script so the template can read
  // them. They're tiny enough that the per-instance cost is moot.
  const eyeOpenPath =
    "<path d='M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z'/><circle cx='12' cy='12' r='3'/>";
  const eyeShutPath =
    "<path d='m3 3 18 18'/><path d='M10.5 5.2A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4'/><path d='M6.6 6.6A17 17 0 0 0 2 12s3.5 7 10 7c1.5 0 2.9-.3 4.1-.8'/><path d='M14.1 14.1a3 3 0 0 1-4.2-4.2'/>";

  function isErrEnv(name: string): boolean {
    const n = name.toLowerCase();
    return n === "prod" || n === "production";
  }
  function isWarnEnv(name: string): boolean {
    const n = name.toLowerCase();
    return n === "stage" || n === "staging";
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
  <form
    method="dialog"
    class="card"
    onsubmit={(e) => {
      e.preventDefault();
      void submit();
    }}
  >
    <header class="head">
      <div class="lock-icon">
        <Icon d={IC.lock} size={18} />
      </div>
      <div class="title-block">
        <div class="title">
          Unlock {env?.name ?? "environment"}
        </div>
        <div class="msg">
          Secrets in
          <span class="mono mono-pill">env://{env?.name ?? "…"}</span>
          are encrypted with your master password. Apiovnia decrypts them in
          memory for this session only — nothing is sent off-device.
        </div>
      </div>
    </header>

    {#if env}
      <div class="env-card">
        <span class="env-dot" class:err={isErrEnv(env.name)} class:warn={isWarnEnv(env.name)}></span>
        <div class="env-meta">
          <div class="env-name">{env.name}</div>
          <div class="mono env-sub">encrypted environment · locked until you unlock</div>
        </div>
        <span class="locked-pill">locked</span>
      </div>
    {/if}

    <div class="body">
      <label class="lbl" for="ap-unlock-pw">Master password</label>
      <div class="field" class:has-error={errMsg}>
        <span class="field-icon"><Icon d={IC.lock} size={13} /></span>
        <input
          id="ap-unlock-pw"
          bind:this={inputEl}
          bind:value={password}
          class="mono field-input"
          type={show ? "text" : "password"}
          placeholder="••••••••••••"
          autocomplete="off"
          disabled={working}
          onkeydown={onKeydown}
        />
        <button
          type="button"
          class="ap-btn icon sm ghost field-eye"
          onclick={() => (show = !show)}
          title={show ? "Hide password" : "Reveal password"}
        >
          <Icon d={show ? eyeOpenPath : eyeShutPath} size={13} />
        </button>
      </div>

      {#if errMsg}
        <div class="err">{errMsg}</div>
      {/if}
    </div>

    <footer class="foot">
      <button type="button" class="ap-btn ghost" onclick={onClose} disabled={working}>
        Cancel <span class="ap-kbd">esc</span>
      </button>
      <div class="grow"></div>
      <button type="submit" class="ap-btn cta" disabled={!password || working}>
        <Icon d={IC.unlock} size={13} />
        <span>{working ? "Unlocking…" : "Unlock"}</span>
        {#if !working}<span class="ap-kbd">↵</span>{/if}
      </button>
    </footer>
  </form>
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
    width: 420px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow:
      0 24px 60px rgba(0, 0, 0, 0.55),
      0 0 0 1px rgba(245, 158, 11, 0.08),
      0 0 40px rgba(245, 158, 11, 0.06);
    overflow: hidden;
    font-family: var(--ui);
  }

  .head {
    padding: 22px 22px 12px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    align-items: flex-start;
  }
  .lock-icon {
    width: 38px;
    height: 38px;
    border-radius: 9px;
    background: linear-gradient(
      180deg,
      rgba(245, 158, 11, 0.18),
      rgba(245, 158, 11, 0.06)
    );
    border: 1px solid rgba(245, 158, 11, 0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.04),
      0 0 24px rgba(245, 158, 11, 0.12);
  }
  .title-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .title {
    font-size: 16px;
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.012em;
  }
  .msg {
    font-size: 12.5px;
    color: var(--fg-muted);
    line-height: 1.5;
    max-width: 360px;
  }
  .mono-pill {
    color: var(--fg-dim);
  }

  .env-card {
    margin: 0 22px;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11.5px;
  }
  .env-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--fg-faint);
    flex-shrink: 0;
  }
  .env-dot.err {
    background: var(--err);
  }
  .env-dot.warn {
    background: var(--warn);
  }
  .env-meta {
    flex: 1;
    min-width: 0;
  }
  .env-name {
    color: var(--fg);
    font-weight: 500;
  }
  .env-sub {
    color: var(--fg-faint);
    font-size: 10.5px;
    margin-top: 2px;
  }
  .locked-pill {
    font-size: 10px;
    color: var(--accent);
    background: var(--accent-bg);
    border: 1px solid var(--accent-bd);
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 600;
  }

  .body {
    padding: 16px 22px 12px;
  }
  .lbl {
    display: block;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 6px;
  }
  .field {
    display: flex;
    align-items: center;
    height: 36px;
    background: var(--bg);
    border: 1px solid var(--accent-bd);
    border-radius: 7px;
    padding: 0 4px 0 10px;
    box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.1);
    transition: border-color 0.12s, box-shadow 0.12s;
  }
  .field.has-error {
    border-color: var(--err);
    box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.1);
  }
  .field-icon {
    color: var(--accent);
    margin-right: 8px;
    display: inline-flex;
  }
  .field-input {
    flex: 1;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--fg);
    font-size: 13px;
    letter-spacing: 0.04em;
  }
  .field-eye {
    width: 28px;
    height: 28px;
  }
  .err {
    margin-top: 8px;
    font-size: 11.5px;
    color: var(--err);
  }

  .foot {
    padding: 14px 22px 18px;
    display: flex;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-2);
  }
  .foot .ap-btn {
    height: 32px;
    padding: 0 14px;
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
</style>
