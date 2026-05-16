<!--
  Singleton host — mount once at the App root. Watches the global `dialogs`
  store and renders whichever dialog is currently open.
-->
<script lang="ts">
  import { dialogs } from "$lib/stores/dialogs.svelte";
  import PromptModal from "./PromptModal.svelte";
  import ConfirmModal from "./ConfirmModal.svelte";
</script>

{#if dialogs.current}
  {#if dialogs.current.kind === "prompt"}
    {@const d = dialogs.current}
    <PromptModal
      opts={d.opts}
      onResolve={(v) => {
        d.resolve(v);
        dialogs._close();
      }}
    />
  {:else}
    {@const d = dialogs.current}
    <ConfirmModal
      opts={d.opts}
      onResolve={(v) => {
        d.resolve(v);
        dialogs._close();
      }}
    />
  {/if}
{/if}
