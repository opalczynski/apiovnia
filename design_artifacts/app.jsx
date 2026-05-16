// Top-level app — lays the four artifacts out on a DesignCanvas

const APIOVNIA_INTRO = `Apiovnia is a desktop REST API client for solo devs.
Local SQLite, no team sync, keyboard-first.
Linear/Raycast-grade craft, single amber accent.`;

function ApiovniaApp() {
  return (
    <DesignCanvas>
      <DCSection
        id="overview"
        title="01 · Main shell"
        subtitle="Trójpanelowy layout · projekt UDL · request POST /auth/login, response 200"
      >
        <DCArtboard id="shell" label="Main shell · 1400×900" width={1400} height={900}>
          <MainShell />
        </DCArtboard>
      </DCSection>

      <DCSection
        id="env"
        title="02 · Env Overrides"
        subtitle="Patch-on-request model — base value as placeholder, override below; amber dot on changed fields"
      >
        <DCArtboard id="env" label="Env Overrides tab · 1080×780" width={1080} height={780}>
          <EnvOverridesPanel />
        </DCArtboard>
      </DCSection>

      <DCSection
        id="response"
        title="03 · Response viewer (Pretty)"
        subtitle="Collapsible JSON, syntax highlighting, hover-copy per value, in-response search (⌘F)"
      >
        <DCArtboard id="response" label="Response · Pretty · 980×740" width={980} height={740}>
          <ResponseViewer />
        </DCArtboard>
      </DCSection>

      <DCSection
        id="unlock"
        title="04 · Master password unlock"
        subtitle="Modal nad zdezfokusowanym shellem — pojawia się przy próbie użycia env prod 🔒"
      >
        <DCArtboard id="unlock" label="Unlock modal · 1100×760" width={1100} height={760}>
          <UnlockModal />
        </DCArtboard>
      </DCSection>
    </DesignCanvas>
  );
}

const root = ReactDOM.createRoot(document.getElementById("root"));
root.render(<ApiovniaApp />);
