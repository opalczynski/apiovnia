<script lang="ts">
  import { onMount } from "svelte";

  import TitleBar from "$lib/components/layout/TitleBar.svelte";
  import ThreePanelLayout from "$lib/components/layout/ThreePanelLayout.svelte";
  import ProjectsPanel from "$lib/components/panels/ProjectsPanel.svelte";
  import RequestsPanel from "$lib/components/panels/RequestsPanel.svelte";
  import DetailPanel from "$lib/components/panels/DetailPanel.svelte";
  import DialogsHost from "$lib/components/modals/DialogsHost.svelte";
  import UnlockEnvModal from "$lib/components/modals/UnlockEnvModal.svelte";
  import CommandPalette from "$lib/components/modals/CommandPalette.svelte";
  import EnvManageModal from "$lib/components/modals/EnvManageModal.svelte";
  import SetEnvPasswordModal from "$lib/components/modals/SetEnvPasswordModal.svelte";
  import ToastHost from "$lib/components/ToastHost.svelte";
  import OpLogHost from "$lib/components/OpLogHost.svelte";
  import HistoryPanel from "$lib/components/panels/HistoryPanel.svelte";
  import OnboardingOverlay from "$lib/components/OnboardingOverlay.svelte";
  import SettingsModal from "$lib/components/modals/SettingsModal.svelte";
  import { app } from "$lib/stores/app.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { installKeymap } from "$lib/keymap";

  onMount(() => {
    void app.loadAll();
    return installKeymap();
  });

  // Breadcrumb mirrors the active selection. Falls back to a single segment
  // when nothing's open.
  const crumbs = $derived.by(() => {
    const out: string[] = [];
    if (app.activeProject) out.push(app.activeProject.name);
    if (app.activeCollection) out.push(app.activeCollection.name);
    if (app.activeRequest) out.push(app.activeRequest.name);
    return out.length > 0 ? out : ["Apiovnia"];
  });
</script>

<div class="root">
  <TitleBar {crumbs} />
  {#if app.error}
    <div class="error-bar">{app.error}</div>
  {/if}
  <ThreePanelLayout>
    {#snippet left()}
      <ProjectsPanel />
    {/snippet}
    {#snippet middle()}
      <RequestsPanel />
    {/snippet}
    {#snippet right()}
      <DetailPanel />
    {/snippet}
  </ThreePanelLayout>

  <DialogsHost />

  {#if app.unlockPrompt}
    {@const prompt = app.unlockPrompt}
    <UnlockEnvModal
      envId={prompt.envId}
      retry={prompt.retry}
      onClose={() => app.dismissUnlockPrompt()}
    />
  {/if}

  {#if app.commandPaletteOpen}
    <CommandPalette onClose={() => app.closePalette()} />
  {/if}

  <EnvManageModal open={app.envManageOpen} onClose={() => app.closeEnvManage()} />

  {#if app.envPasswordSetupId}
    {@const id = app.envPasswordSetupId}
    <SetEnvPasswordModal envId={id} onClose={() => app.closeEnvPasswordSetup()} />
  {/if}

  {#if app.historyPanelOpen}
    <HistoryPanel onClose={() => app.closeHistoryPanel()} />
  {/if}

  {#if !app.loading && app.projects.length === 0}
    <OnboardingOverlay />
  {/if}

  {#if settings.open}
    <SettingsModal onClose={() => (settings.open = false)} />
  {/if}

  <ToastHost />
  <OpLogHost />
</div>

<style>
  .root {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
  }
  .error-bar {
    padding: 6px 12px;
    background: color-mix(in srgb, var(--err) 14%, transparent);
    color: var(--err);
    font-size: 11.5px;
    border-bottom: 1px solid color-mix(in srgb, var(--err) 30%, transparent);
    font-family: var(--mono);
  }
</style>
