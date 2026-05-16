<!--
  Select — custom dropdown picker. Replaces the native <select> for our
  method/body-type/auth-type pickers so we control look + behaviour.

  Keyboard:
    - Trigger gets focus → Space/Enter/↓ opens
    - ↑/↓ navigates highlighted item, Enter selects, Esc closes
    - Type-ahead: pressing a letter jumps to next matching label
-->
<script module lang="ts">
  export type SelectOption<V extends string> = {
    value: V;
    label: string;
  };
</script>

<script lang="ts" generics="T extends string">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import Popover from "./Popover.svelte";
  import type { Snippet } from "svelte";

  type Props = {
    value: T;
    options: SelectOption<T>[];
    onChange: (v: T) => void | Promise<void>;
    /** Custom snippet to render the picked value in the trigger. */
    triggerLabel?: Snippet<[SelectOption<T>]>;
    /** Custom snippet to render a list option. */
    optionLabel?: Snippet<[SelectOption<T>]>;
    placeholder?: string;
    disabled?: boolean;
    /** ARIA label for the trigger button. */
    ariaLabel?: string;
    /** CSS class forwarded to the trigger. */
    class?: string;
    /** Match menu width to trigger button. */
    matchAnchorWidth?: boolean;
  };

  let {
    value,
    options,
    onChange,
    triggerLabel,
    optionLabel,
    placeholder = "Select…",
    disabled = false,
    ariaLabel,
    class: className = "",
    matchAnchorWidth = false,
  }: Props = $props();

  let open = $state(false);
  let triggerEl: HTMLButtonElement | undefined = $state();
  let highlighted = $state(0);

  let current = $derived(options.find((o) => o.value === value));

  function openMenu() {
    if (disabled) return;
    open = true;
    highlighted = Math.max(
      0,
      options.findIndex((o) => o.value === value),
    );
  }

  async function pick(v: T) {
    open = false;
    if (v !== value) await onChange(v);
    triggerEl?.focus();
  }

  function onTriggerKey(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown") {
      e.preventDefault();
      openMenu();
    }
  }

  function onListKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      highlighted = (highlighted + 1) % options.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      highlighted = (highlighted - 1 + options.length) % options.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      const o = options[highlighted];
      if (o) void pick(o.value);
    } else if (e.key === "Escape") {
      e.preventDefault();
      open = false;
      triggerEl?.focus();
    } else if (/^[a-z0-9]$/i.test(e.key)) {
      const idx = options.findIndex((o) =>
        o.label.toLowerCase().startsWith(e.key.toLowerCase()),
      );
      if (idx >= 0) highlighted = idx;
    }
  }
</script>

<button
  bind:this={triggerEl}
  type="button"
  class="trigger {className}"
  class:disabled
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-label={ariaLabel}
  {disabled}
  onclick={openMenu}
  onkeydown={onTriggerKey}
>
  {#if current && triggerLabel}
    {@render triggerLabel(current)}
  {:else if current}
    <span class="label">{current.label}</span>
  {:else}
    <span class="placeholder">{placeholder}</span>
  {/if}
  <span class="caret"><Icon d={IC.caret} size={12} /></span>
</button>

<Popover anchor={triggerEl} bind:open {matchAnchorWidth}>
  <ul
    class="list"
    role="listbox"
    tabindex="-1"
    onkeydown={onListKey}
    aria-activedescendant={`opt-${highlighted}`}
  >
    {#each options as o, i (o.value)}
      <li
        id={`opt-${i}`}
        role="option"
        aria-selected={o.value === value}
        class="opt"
        class:highlight={i === highlighted}
        class:selected={o.value === value}
        onmouseenter={() => (highlighted = i)}
        onclick={() => void pick(o.value)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            void pick(o.value);
          }
        }}
      >
        {#if optionLabel}
          {@render optionLabel(o)}
        {:else}
          <span class="label">{o.label}</span>
        {/if}
        {#if o.value === value}
          <span class="check"><Icon d={IC.check} size={12} /></span>
        {/if}
      </li>
    {/each}
  </ul>
</Popover>

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-ctrl);
    color: var(--fg);
    font: 500 12px/1 var(--ui);
    cursor: pointer;
    transition:
      border-color 0.12s,
      background 0.12s;
  }
  .trigger:hover:not(.disabled) {
    border-color: var(--border-strong);
    background: var(--hover);
  }
  .trigger:focus-visible {
    outline: none;
    border-color: var(--accent-bd);
    box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.12);
  }
  .trigger.disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .label {
    flex: 1;
  }
  .placeholder {
    color: var(--fg-faint);
  }
  .caret {
    color: var(--fg-muted);
    display: inline-flex;
    align-items: center;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 4px;
    max-height: 280px;
    overflow: auto;
    min-width: 140px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 8px;
    border-radius: 5px;
    color: var(--fg-dim);
    cursor: pointer;
    font-size: 12px;
  }
  .opt.highlight {
    background: var(--hover);
    color: var(--fg);
  }
  .opt.selected {
    color: var(--fg);
  }
  .opt .check {
    color: var(--accent);
    display: inline-flex;
  }
</style>
