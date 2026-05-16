<!--
  Segment — horizontal segmented picker (radio group, exclusive). Used for
  the body-type picker (JSON/Form/Raw/None) and other small enumerations.
-->
<script lang="ts" generics="T extends string">
  type Option<V extends string> = {
    value: V;
    label: string;
    disabled?: boolean;
  };

  type Props = {
    value: T;
    options: Option<T>[];
    onChange: (v: T) => void | Promise<void>;
    size?: "sm" | "md";
    /** Optional ARIA label for the group. */
    ariaLabel?: string;
  };

  const {
    value,
    options,
    onChange,
    size = "sm",
    ariaLabel,
  }: Props = $props();

  function pick(o: Option<T>) {
    if (o.disabled || o.value === value) return;
    void onChange(o.value);
  }
</script>

<div class="seg {size}" role="radiogroup" aria-label={ariaLabel}>
  {#each options as o (o.value)}
    {@const active = o.value === value}
    <button
      type="button"
      class="opt"
      class:active
      disabled={o.disabled}
      role="radio"
      aria-checked={active}
      onclick={() => pick(o)}
    >
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    gap: 2px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 2px;
  }
  .opt {
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg-muted);
    border-radius: 4px;
    font: 500 var(--seg-fs, 11px) / 1 var(--ui);
    cursor: pointer;
    transition:
      background 0.12s,
      color 0.12s,
      border-color 0.12s;
  }
  .seg.sm {
    --seg-fs: 10.5px;
  }
  .seg.md {
    --seg-fs: 12px;
  }
  .seg.sm .opt {
    height: 20px;
    padding: 0 8px;
  }
  .seg.md .opt {
    height: 26px;
    padding: 0 12px;
  }
  .opt:hover:not(:disabled):not(.active) {
    color: var(--fg-dim);
  }
  .opt.active {
    background: var(--bg);
    color: var(--fg);
    border-color: var(--border-strong);
  }
  .opt:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
</style>
