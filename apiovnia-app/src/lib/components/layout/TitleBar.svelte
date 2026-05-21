<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { app } from "$lib/stores/app.svelte";

  type Props = {
    crumbs?: string[];
  };

  const { crumbs = ["UDL", "Auth", "Login"] }: Props = $props();
</script>

<header class="titlebar">
  <!-- Small honeycomb logo — mirrors the app icon's mid-size variant
       (outline hex + filled centre dot). Stays at 16 px to fit the
       32 px-tall title bar without crowding the breadcrumb. -->
  <div class="brand" title="Apiovnia">
    <svg
      class="logo"
      viewBox="0 0 100 100"
      width="18"
      height="18"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <polygon
        points="50,18 78,34 78,66 50,82 22,66 22,34"
        fill="none"
        stroke="#F59E0B"
        stroke-width="7"
        stroke-linejoin="round"
      />
      <circle cx="50" cy="50" r="11" fill="#F59E0B" />
    </svg>
    <span class="brand-name">Apiovnia</span>
  </div>

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
    <!-- Search button — visible alias for the command palette. Click opens
         the palette (same target as ⌘P). The kbd hint keeps the shortcut
         discoverable next to its action. -->
    <button
      class="ap-btn ghost sm"
      type="button"
      title="Open command palette"
      onclick={() => app.openPalette()}
    >
      <Icon d={IC.search} />
      <span>Search</span>
      <span class="ap-kbd">⌘P</span>
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

  .brand {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-left: 12px;
    flex-shrink: 0;
    user-select: none;
  }
  .logo {
    flex-shrink: 0;
    display: block;
  }
  .brand-name {
    font: 600 11.5px/1 var(--ui);
    color: var(--fg);
    letter-spacing: -0.005em;
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
