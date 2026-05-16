<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";

  type Props = {
    crumbs?: string[];
  };

  const { crumbs = ["UDL", "Auth", "Login"] }: Props = $props();
</script>

<header class="titlebar">
  <div class="crumbs-wrap">
    <div class="crumbs">
      {#each crumbs as label, i (i)}
        {#if i > 0}<span class="sep">/</span>{/if}
        <span class="crumb" class:current={i === crumbs.length - 1}>
          {label}
        </span>
      {/each}
    </div>
  </div>

  <div class="actions">
    <button class="ap-btn ghost sm">
      <Icon d={IC.search} />
      <span>Search</span>
      <span class="ap-kbd">⌘K</span>
    </button>
    <button class="ap-btn ghost icon sm" title="Settings">
      <Icon d={IC.settings} />
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: 36px;
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    padding-right: 10px;
    flex-shrink: 0;
  }

  .crumbs-wrap {
    flex: 1;
    display: flex;
    justify-content: center;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    font-size: 11px;
    color: var(--fg-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    min-width: 320px;
    justify-content: center;
  }
  .crumb {
    color: var(--fg-dim);
  }
  .crumb.current {
    color: var(--fg);
  }
  .sep {
    color: var(--fg-faint);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
