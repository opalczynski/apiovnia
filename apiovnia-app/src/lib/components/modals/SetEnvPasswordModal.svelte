<!--
  SetEnvPasswordModal — picks the master password the first time an env is
  sealed. Confirms the password is non-empty and matches itself, then asks
  the backend to flip the env to encrypted (which migrates every existing
  variable + override in one transaction).

  Used by EnvManageModal's "Lock with password" action.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import * as ipc from "$lib/api/ipc";
  import { app } from "$lib/stores/app.svelte";
  import type { EnvironmentId, PasswordStrength } from "$lib/types/domain";

  type Props = {
    envId: EnvironmentId;
    onClose: () => void;
  };

  const { envId, onClose }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let pwInputEl: HTMLInputElement | undefined = $state();
  let pw = $state("");
  let confirmPw = $state("");
  let working = $state(false);
  let errMsg = $state<string | null>(null);

  /** Latest scoring snapshot from the backend; null until the user has typed
   *  something and the debounced IPC has returned. */
  let strength = $state<PasswordStrength | null>(null);

  /** Pro-user escape hatch — relaxes the policy gate to "any non-empty
   *  password". Off by default; user has to explicitly opt in. */
  let bypassPolicy = $state(false);

  const env = $derived(app.envs.find((e) => e.id === envId) ?? null);
  const mismatch = $derived(
    pw.length > 0 && confirmPw.length > 0 && pw !== confirmPw,
  );

  /** Sentinel: gate the submit button. With bypass on, only the non-empty
   *  + matching-confirm checks remain. */
  const canSubmit = $derived(
    !working &&
      !mismatch &&
      pw.length > 0 &&
      confirmPw.length > 0 &&
      (bypassPolicy || strength?.meetsPolicy === true),
  );

  $effect(() => {
    if (!dialogEl) return;
    dialogEl.showModal();
    queueMicrotask(() => pwInputEl?.focus());
  });

  // Live scoring — debounce 120ms so we don't trip an IPC per keystroke.
  // Race-protect by stamping each call and ignoring stale results.
  let scoreSeq = 0;
  let scoreTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    // Track `pw` for reactivity.
    const current = pw;
    if (scoreTimer) clearTimeout(scoreTimer);
    if (current.length === 0) {
      strength = null;
      return;
    }
    const my = ++scoreSeq;
    scoreTimer = setTimeout(() => {
      void ipc.scorePassword(current).then((s) => {
        // Drop if a newer call has already kicked off — UI stays consistent.
        if (my === scoreSeq) strength = s;
      });
    }, 120);
  });

  async function submit() {
    if (!canSubmit) return;
    working = true;
    errMsg = null;
    try {
      await app.enableEnvEncryption(envId, pw, bypassPolicy);
      working = false;
      onClose();
    } catch (e) {
      working = false;
      errMsg = e instanceof Error ? e.message : String(e);
    }
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onClose();
  }

  // small inline warning icon — used in the body hint
  const dangerPath =
    "<path d='M12 9v4M12 17h.01'/><path d='M10.3 3.9 2.7 17.5A2 2 0 0 0 4.4 20.5h15.2a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z'/>";
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
      <div>
        <div class="title">
          Lock {env?.name ?? "environment"} with a master password
        </div>
        <div class="msg">
          Every variable value and override field in this environment will be
          encrypted with <b>AES-256-GCM</b> using a key derived from your
          password (<b>Argon2id</b>). The password is never stored — losing it
          means the env is unrecoverable.
        </div>
      </div>
    </header>

    <div class="body">
      <label class="lbl" for="ap-pw">Master password</label>
      <input
        id="ap-pw"
        bind:this={pwInputEl}
        bind:value={pw}
        class="ap-input field mono"
        type="password"
        placeholder="••••••••••••"
        autocomplete="new-password"
        disabled={working}
      />

      <!-- Live strength meter + crack-time line. Hidden until the user has
           typed something so the empty state stays clean. -->
      {#if pw.length > 0}
        <div class="meter" aria-label="Password strength">
          {#each Array(5) as _, i}
            <span
              class="seg"
              class:on={strength && i <= strength.score}
              data-rank={Math.min(strength?.score ?? -1, 4)}
            ></span>
          {/each}
        </div>
        <div class="meter-line">
          {#if !strength}
            <span class="meter-label muted">Scoring…</span>
          {:else if !strength.longEnough}
            <span class="meter-label muted">
              {pw.length}/8 characters — keep going
            </span>
          {:else}
            <span class="meter-label" data-rank={strength.score}>
              {strength.label}
            </span>
            <span class="meter-sep">·</span>
            <span class="meter-crack">
              Cracking time:
              <b>~{strength.crackTimeDisplay}</b>
            </span>
          {/if}
        </div>
        {#if strength && !strength.meetsPolicy && strength.longEnough}
          <div class="hint err">
            {strength.warning ??
              strength.suggestions[0] ??
              `Too easy to guess — aim for "Strong" or better.`}
          </div>
        {/if}
      {/if}

      <label class="lbl" for="ap-pw2">Confirm</label>
      <input
        id="ap-pw2"
        bind:value={confirmPw}
        class="ap-input field mono"
        class:err={mismatch}
        type="password"
        placeholder="••••••••••••"
        autocomplete="new-password"
        disabled={working}
      />

      {#if mismatch}
        <div class="hint err">Passwords don't match.</div>
      {:else}
        <div class="recover-warn">
          <span class="recover-icon"><Icon d={dangerPath} size={14} /></span>
          <span>
            <b>Apiovnia cannot recover this password.</b>
            Lose it and the environment's secrets are gone — there's no
            reset link, no master backup, no support email.
          </span>
        </div>
      {/if}

      <!-- Pro-user escape hatch. Discreet but findable; off by default. -->
      <label class="pro-toggle" class:on={bypassPolicy}>
        <input
          type="checkbox"
          bind:checked={bypassPolicy}
          disabled={working}
        />
        <span class="pro-text">
          I'm a pro user — bypass password policy, I know what I'm doing.
        </span>
      </label>

      {#if errMsg}
        <div class="hint err">{errMsg}</div>
      {/if}
    </div>

    <footer class="foot">
      <button type="button" class="ap-btn ghost" onclick={onClose} disabled={working}>
        Cancel <span class="ap-kbd">esc</span>
      </button>
      <div class="grow"></div>
      <button
        type="submit"
        class="ap-btn cta"
        disabled={!canSubmit}
        title={!canSubmit && pw.length > 0 && strength && !strength.meetsPolicy
          ? "Password is too weak — aim for Strong or better"
          : undefined}
      >
        <Icon d={IC.lock} size={13} />
        <span>{working ? "Encrypting…" : "Lock environment"}</span>
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
    width: 440px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow:
      0 24px 60px rgba(0, 0, 0, 0.55),
      0 0 0 1px rgba(245, 158, 11, 0.08);
    overflow: hidden;
    font-family: var(--ui);
  }

  .head {
    padding: 20px 22px 12px;
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .lock-icon {
    width: 36px;
    height: 36px;
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
    flex-shrink: 0;
  }
  .title {
    font-size: 14.5px;
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .msg {
    margin-top: 6px;
    font-size: 12px;
    color: var(--fg-muted);
    line-height: 1.5;
  }
  .msg b {
    color: var(--fg);
    font-weight: 600;
  }

  .body {
    padding: 10px 22px 14px;
  }
  .lbl {
    display: block;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin: 12px 0 6px;
  }
  .field {
    width: 100%;
    height: 34px;
    font-size: 13px;
    padding: 0 10px;
    letter-spacing: 0.04em;
  }
  .field:focus {
    border-color: var(--accent-bd);
    box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.1);
  }
  .field.err {
    border-color: var(--err);
    box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.1);
  }
  .hint {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--fg-faint);
    line-height: 1.5;
  }
  .hint.err {
    color: var(--err);
  }

  /* Loud "no recovery" banner — replaces the timid grey hint that nobody
     reads. Amber wash + bold opener gets the user's attention without
     looking like a hard error (we save red for actual blockers). */
  .recover-warn {
    margin-top: 12px;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 9px 11px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-left-width: 3px;
    border-radius: 6px;
    font-size: 11.5px;
    color: var(--fg);
    line-height: 1.5;
  }
  .recover-warn b {
    color: var(--accent);
    font-weight: 600;
  }
  .recover-icon {
    color: var(--accent);
    flex-shrink: 0;
    display: inline-flex;
    margin-top: 1px;
  }

  /* Strength meter — five segments coloured by zxcvbn score. Empty segments
     stay at the surface-2 colour so the bar always shows the full width. */
  .meter {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    margin: 8px 0 4px;
  }
  .seg {
    height: 4px;
    border-radius: 2px;
    background: var(--surface-2);
    transition: background 0.15s;
  }
  /* When a segment is "on", paint it the colour of the *score's tier* — so
     all lit segments share one colour, making the meter feel cohesive
     rather than rainbow-vomit. */
  .seg.on[data-rank="0"] { background: var(--err); }
  .seg.on[data-rank="1"] { background: var(--err); opacity: 0.85; }
  .seg.on[data-rank="2"] { background: var(--warn); }
  .seg.on[data-rank="3"] { background: var(--ok); }
  .seg.on[data-rank="4"] { background: var(--ok); box-shadow: 0 0 4px color-mix(in srgb, var(--ok) 50%, transparent); }

  .meter-line {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--fg-muted);
    line-height: 1.5;
    min-height: 18px;
  }
  .meter-label {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-size: 10px;
  }
  .meter-label.muted {
    color: var(--fg-faint);
    text-transform: none;
    letter-spacing: 0;
    font-size: 11px;
    font-weight: 500;
  }
  .meter-label[data-rank="0"],
  .meter-label[data-rank="1"] { color: var(--err); }
  .meter-label[data-rank="2"] { color: var(--warn); }
  .meter-label[data-rank="3"] { color: var(--ok); }
  .meter-label[data-rank="4"] { color: var(--ok); }
  .meter-sep {
    color: var(--fg-faint);
  }
  .meter-crack {
    color: var(--fg-muted);
  }
  .meter-crack b {
    color: var(--fg);
    font-weight: 600;
  }

  /* Pro-user bypass checkbox. Discreet by default, tints amber when on so
     the user sees they've crossed the policy line. */
  .pro-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px 10px;
    border: 1px dashed var(--border);
    border-radius: 6px;
    background: var(--bg);
    cursor: pointer;
    user-select: none;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }
  .pro-toggle:hover {
    border-color: var(--border-strong);
  }
  .pro-toggle.on {
    border-color: var(--accent-bd);
    background: var(--accent-bg);
  }
  .pro-toggle input[type="checkbox"] {
    width: 14px;
    height: 14px;
    margin: 0;
    accent-color: var(--accent);
    flex-shrink: 0;
    cursor: pointer;
  }
  .pro-text {
    font-size: 11px;
    color: var(--fg-muted);
    line-height: 1.4;
  }
  .pro-toggle.on .pro-text {
    color: var(--accent);
  }

  .foot {
    padding: 12px 22px 16px;
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
