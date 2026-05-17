/**
 * Global keyboard shortcuts.
 *
 * One window-level `keydown` listener. We dispatch only on the mod-key
 * combos we own; everything else bubbles unchanged so the browser /
 * CodeMirror / inputs keep working normally.
 *
 * Cross-platform: `Cmd` on macOS, `Ctrl` everywhere else.
 *
 * Context-scoped shortcuts (⌘K focus filter inside ProjectsPanel, ⌘Enter
 * Send inside UrlBar, ⌘F search inside JsonView) live with the components
 * they affect — we don't centralise those because they want to know the
 * caller's local state.
 */

import { app } from "$lib/stores/app.svelte";
import { dialogs } from "$lib/stores/dialogs.svelte";

const isMac =
  typeof navigator !== "undefined" &&
  /mac|iphone|ipad|ipod/i.test(navigator.platform);

/** True iff this event has the platform mod key held (and no other mods). */
function modOnly(e: KeyboardEvent): boolean {
  const hasMod = isMac ? e.metaKey : e.ctrlKey;
  // Allow Shift to coexist for future chords; explicitly reject the *other*
  // platform mod so ⌘+Ctrl combos don't fire by accident.
  const otherMod = isMac ? e.ctrlKey : e.metaKey;
  return hasMod && !e.altKey && !otherMod;
}

/**
 * Hook the global listeners. Returns an `unsubscribe` for the caller's
 * `$effect` cleanup — App.svelte calls this once on mount.
 */
export function installKeymap(): () => void {
  function listener(e: KeyboardEvent) {
    if (!modOnly(e)) return;

    const k = e.key.toLowerCase();

    // ⌘P — toggle command palette. Works from anywhere, including inside
    // inputs — we want it even when the URL bar has focus.
    // (Phase 9.5: swapped from ⌘K so ⌘K can match Postman/Insomnia muscle
    // memory for "search the sidebar".)
    if (k === "p") {
      e.preventDefault();
      if (app.commandPaletteOpen) app.closePalette();
      else app.openPalette();
      return;
    }

    // ⌘N — new request in active collection. Suppressed when typing in an
    // input/textarea/contenteditable so it doesn't fight with "type N".
    if (k === "n") {
      if (isEditableTarget(e.target)) return;
      e.preventDefault();
      void newRequestPrompt();
      return;
    }

    // ⌘1 / ⌘2 / ⌘3 — focus left/middle/right panel. Targets carry a
    // `data-focus-target` attribute; we just plumb that through. Works
    // even inside other inputs — switching focus IS the intent.
    if (k === "1" || k === "2" || k === "3") {
      const target =
        k === "1" ? "left" : k === "2" ? "mid" : "right";
      const el = document.querySelector<HTMLElement>(
        `[data-focus-target="${target}"]`,
      );
      if (el) {
        e.preventDefault();
        el.focus();
        if (el instanceof HTMLInputElement) el.select();
      }
      return;
    }
  }

  window.addEventListener("keydown", listener);
  return () => window.removeEventListener("keydown", listener);
}

/** Heuristic: is the event firing inside something the user is typing in? */
function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

/**
 * Prompt for a name and create a request in the active collection. Falls
 * back to a friendly "pick a collection first" hint when there's nowhere
 * to put it — saves the user from getting silently nothing.
 */
async function newRequestPrompt(): Promise<void> {
  if (!app.activeCollectionId) {
    await dialogs.confirm({
      title: "No active collection",
      message:
        "Pick a collection in the middle panel (or create one) before adding a request.",
      confirmLabel: "OK",
    });
    return;
  }
  const name = await dialogs.prompt({
    title: "New request",
    placeholder: "e.g. Get user",
    confirmLabel: "Create request",
  });
  if (name) await app.createRequest(name);
}
