<!--
  Tabs — header rail with active underline. The matching content snippet is
  rendered below via `tabContent` keyed by tab id.

  Keyboard: ←/→ to move focus, Enter/Space to select.
-->
<script module lang="ts">
  export type TabSpec<V extends string> = {
    id: V;
    label: string;
    /** Numeric badge next to the label (`null`/0 hides). */
    count?: number | null;
    /** String badge (e.g. body content-type "json"). */
    badge?: string;
    disabled?: boolean;
    /** Show a faint "soon" pill. */
    soon?: boolean;
  };
</script>

<script lang="ts" generics="T extends string">
  import type { Snippet } from "svelte";

  type Props = {
    value: T;
    tabs: TabSpec<T>[];
    onChange: (v: T) => void;
    children: Snippet;
  };

  const { value, tabs, onChange, children }: Props = $props();

  function onKey(e: KeyboardEvent) {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    const active = tabs.filter((t) => !t.disabled);
    const idx = active.findIndex((t) => t.id === value);
    if (idx < 0) return;
    const next = active[(idx + (e.key === "ArrowRight" ? 1 : -1) + active.length) % active.length];
    if (next) onChange(next.id);
  }
</script>

<div class="ap-tabs" role="tablist" tabindex="-1" onkeydown={onKey}>
  {#each tabs as t (t.id)}
    {@const active = t.id === value}
    <button
      type="button"
      class="ap-tab"
      class:active
      role="tab"
      aria-selected={active}
      tabindex={active ? 0 : -1}
      disabled={t.disabled}
      onclick={() => !t.disabled && onChange(t.id)}
    >
      <span>{t.label}</span>
      {#if t.count}<span class="count">{t.count}</span>{/if}
      {#if t.badge}
        <span class="count mono badge">{t.badge}</span>
      {/if}
      {#if t.soon}<span class="count" style="opacity:.6">soon</span>{/if}
    </button>
  {/each}
</div>

{@render children()}

<style>
  .badge {
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .ap-tab[disabled] {
    cursor: not-allowed;
    opacity: 0.55;
  }
</style>
