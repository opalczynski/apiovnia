/**
 * Promise-based dialogs (prompt / confirm) with our own UI.
 *
 * One global queue — only one dialog is open at a time. Components import
 * `dialogs.prompt(...)` / `dialogs.confirm(...)` and `await` the result.
 *
 * The `<DialogsHost />` component reads `dialogs.current` and renders the
 * matching modal. Resolving here closes it.
 */

export type PromptOptions = {
  title: string;
  message?: string;
  placeholder?: string;
  defaultValue?: string;
  confirmLabel?: string;
  /** `"password"` renders the input as `type="password"` (masked, no
   *  autocomplete). Defaults to `"text"`. */
  kind?: "text" | "password";
};

export type ConfirmOptions = {
  title: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
};

type PromptState = {
  kind: "prompt";
  opts: PromptOptions;
  resolve: (v: string | null) => void;
};

type ConfirmState = {
  kind: "confirm";
  opts: ConfirmOptions;
  resolve: (v: boolean) => void;
};

type DialogState = PromptState | ConfirmState;

const state = $state<{ current: DialogState | null }>({ current: null });

function open(s: DialogState) {
  // If a dialog is already open, cancel it first so callers always settle.
  if (state.current) {
    if (state.current.kind === "prompt") state.current.resolve(null);
    else state.current.resolve(false);
  }
  state.current = s;
}

function close() {
  state.current = null;
}

export const dialogs = {
  get current() {
    return state.current;
  },

  prompt(opts: PromptOptions): Promise<string | null> {
    return new Promise((resolve) => {
      open({ kind: "prompt", opts, resolve });
    });
  },

  confirm(opts: ConfirmOptions): Promise<boolean> {
    return new Promise((resolve) => {
      open({ kind: "confirm", opts, resolve });
    });
  },

  /** Called by the host components after resolving their promise. */
  _close: close,
};
