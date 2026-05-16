// Artifact 2: Env Overrides panel — only the right panel, focused on Env Overrides tab

const { useState: useS2 } = React;

function EnvTab({ name, active, locked, onClick }) {
  return (
    <button
      onClick={onClick}
      style={{
        display: "inline-flex", alignItems: "center", gap: 6,
        height: 28, padding: "0 12px",
        background: active ? "var(--surface-2)" : "transparent",
        border: "1px solid " + (active ? "var(--border-strong)" : "transparent"),
        borderRadius: 6,
        color: active ? "var(--fg)" : "var(--fg-muted)",
        fontSize: 12,
        fontWeight: active ? 600 : 500,
        cursor: "pointer",
        position: "relative",
      }}>
      <span style={{
        width: 6, height: 6, borderRadius: "50%",
        background: locked ? "var(--fg-faint)" : (
          name === "prod" ? "var(--err)" : name === "stage" ? "var(--warn)" : "var(--ok)"
        ),
      }} />
      <span>{name}</span>
      {locked && <span style={{ color: "var(--fg-muted)" }}>{IC.lock}</span>}
    </button>
  );
}

function OverrideField({ label, baseValue, overrideValue, secret, onClear }) {
  const isOverridden = overrideValue !== undefined && overrideValue !== "";
  return (
    <div style={{
      display: "grid",
      gridTemplateColumns: "180px 1fr 1fr 28px",
      gap: 12,
      alignItems: "center",
      padding: "10px 14px",
      borderBottom: "1px solid var(--border-soft)",
      position: "relative",
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
        {isOverridden ? <div className="ap-override-dot" /> : <div style={{ width: 6, flexShrink: 0 }} />}
        <div style={{ minWidth: 0 }}>
          <div className="mono" style={{
            fontSize: 11.5,
            color: isOverridden ? "var(--accent)" : "var(--fg)",
            overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
            fontWeight: isOverridden ? 600 : 500,
          }}>{label}</div>
          <div style={{ fontSize: 10, color: "var(--fg-faint)", marginTop: 2, textTransform: "uppercase", letterSpacing: ".06em" }}>
            {isOverridden ? "overridden in dev" : "inherits base"}
          </div>
        </div>
      </div>

      <div style={{
        height: 28, padding: "0 8px",
        display: "flex", alignItems: "center",
        background: "var(--surface)",
        border: "1px solid var(--border-soft)",
        borderRadius: 6,
        color: "var(--fg-muted)",
        fontFamily: "var(--mono)", fontSize: 11.5,
        overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis",
      }} title={baseValue}>
        <span style={{ color: "var(--fg-faint)", marginRight: 6, fontSize: 10, textTransform: "uppercase", letterSpacing: ".06em" }}>base</span>
        <span>{secret ? "••••••••••••••••" : baseValue}</span>
      </div>

      <div style={{
        height: 28, padding: "0 8px",
        display: "flex", alignItems: "center",
        background: "var(--bg)",
        border: "1px solid " + (isOverridden ? "var(--accent-bd)" : "var(--border)"),
        borderRadius: 6,
        color: isOverridden ? "var(--fg)" : "var(--fg-faint)",
        fontFamily: "var(--mono)", fontSize: 11.5,
        overflow: "hidden", whiteSpace: "nowrap",
        boxShadow: isOverridden ? "0 0 0 2px rgba(245,158,11,0.08)" : "none",
      }}>
        {isOverridden ? (
          <span style={{ overflow: "hidden", textOverflow: "ellipsis" }} title={overrideValue}>
            {secret ? "••••••••••••••••" : overrideValue}
          </span>
        ) : (
          <span style={{ color: "var(--fg-faint)", fontStyle: "italic" }}>
            override for dev…
          </span>
        )}
      </div>

      {isOverridden ? (
        <button className="ap-btn icon sm ghost" title="Clear override" onClick={onClear}>
          {IC.x}
        </button>
      ) : (
        <div style={{ width: 22 }} />
      )}
    </div>
  );
}

function GroupHeader({ title, count, addable }) {
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 8,
      padding: "10px 14px 6px",
      borderBottom: "1px solid var(--border-soft)",
      background: "var(--surface)",
    }}>
      <span style={{ fontSize: 11, fontWeight: 600, color: "var(--fg-dim)", letterSpacing: "-.005em" }}>{title}</span>
      {count !== undefined && <span style={{ fontSize: 10.5, color: "var(--fg-faint)", fontVariantNumeric: "tabular-nums" }}>{count}</span>}
      <div style={{ flex: 1 }} />
      {addable && (
        <button className="ap-btn sm ghost" style={{ height: 22, padding: "0 6px", gap: 4 }}>
          {IC.plus}<span style={{ fontSize: 11 }}>Add</span>
        </button>
      )}
    </div>
  );
}

function EnvOverridesPanel() {
  const [env, setEnv] = useS2("dev");
  return (
    <div className="ap" style={{
      width: "100%", height: "100%",
      background: "var(--surface)",
      display: "flex", flexDirection: "column",
      borderRadius: 10, overflow: "hidden",
      border: "1px solid var(--border-strong)",
      boxShadow: "0 30px 60px rgba(0,0,0,0.4)",
    }}>
      {/* URL bar mini */}
      <div style={{
        display: "flex", alignItems: "center", gap: 6,
        padding: "10px 14px",
        borderBottom: "1px solid var(--border)",
        background: "var(--surface)",
      }}>
        <MethodBadge m="POST" />
        <div className="mono" style={{
          flex: 1, height: 28, padding: "0 10px",
          background: "var(--bg)", border: "1px solid var(--border)", borderRadius: 6,
          display: "flex", alignItems: "center", gap: 0, fontSize: 12,
        }}>
          <span style={{ color: "var(--fg-faint)" }}>https://</span>
          <span style={{ color: "var(--accent)" }}>{"{{base_url}}"}</span>
          <span style={{ color: "var(--fg)" }}>/auth/login</span>
        </div>
        <button className="ap-btn cta" style={{ height: 28 }}>{IC.send}<span>Send</span></button>
      </div>

      {/* Request tabs */}
      <div className="ap-tabs">
        {["Params","Headers","Body","Auth","Env Overrides","Tests"].map((t, i) => (
          <button key={t} className={"ap-tab" + (t === "Env Overrides" ? " active" : "")}>
            <span>{t}</span>
            {t === "Env Overrides" && <span className="count">2</span>}
            {t === "Headers" && <span className="count">3</span>}
            {t === "Params" && <span className="count">2</span>}
          </button>
        ))}
      </div>

      {/* Env switcher row */}
      <div style={{
        display: "flex", alignItems: "center", gap: 4,
        padding: "10px 14px",
        borderBottom: "1px solid var(--border)",
        background: "var(--bg)",
      }}>
        <div style={{ display: "flex", gap: 2, background: "var(--surface-2)", padding: 2, borderRadius: 7, border: "1px solid var(--border)" }}>
          <EnvTab name="dev" active={env === "dev"} onClick={() => setEnv("dev")} />
          <EnvTab name="stage" active={env === "stage"} onClick={() => setEnv("stage")} />
          <EnvTab name="prod" active={env === "prod"} locked onClick={() => setEnv("prod")} />
          <EnvTab name="local-tomek" active={env === "local-tomek"} onClick={() => setEnv("local-tomek")} />
        </div>
        <div style={{ flex: 1 }} />
        <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--fg-muted)" }}>
          <div className="ap-override-dot" />
          <span><b style={{ color: "var(--fg)" }}>2</b> overrides in this env</span>
        </div>
        <button className="ap-btn sm ghost">Diff base</button>
        <button className="ap-btn sm">{IC.plus}<span>Add field override</span></button>
      </div>

      {/* Explainer */}
      <div style={{
        padding: "10px 14px",
        background: "var(--bg)",
        borderBottom: "1px solid var(--border-soft)",
        fontSize: 11.5,
        color: "var(--fg-muted)",
        display: "flex", alignItems: "flex-start", gap: 10,
        lineHeight: 1.45,
      }}>
        <span style={{ color: "var(--accent)", marginTop: 1 }}>{IC.globe}</span>
        <div style={{ flex: 1 }}>
          <span style={{ color: "var(--fg-dim)" }}>Env overrides are patches on this request — </span>
          <span>they replace the base value when this env is active. Unset fields inherit the base. </span>
          <span className="mono" style={{ color: "var(--fg-faint)" }}>request &gt; env override &gt; base</span>
        </div>
        <button className="ap-btn icon sm ghost">{IC.x}</button>
      </div>

      {/* Fields */}
      <div style={{ flex: 1, overflow: "auto" }}>
        <GroupHeader title="Request" />
        <OverrideField
          label="URL"
          baseValue="https://{{base_url}}/auth/login"
          overrideValue="https://api-dev.udl.test/auth/login"
        />
        <OverrideField
          label="method"
          baseValue="POST"
        />

        <GroupHeader title="Headers" count={3} addable />
        <OverrideField
          label="Authorization"
          baseValue="Bearer {{access_token}}"
          overrideValue="Bearer dev_T_4f3a9c2e1b7d"
          secret
        />
        <OverrideField
          label="X-Workspace-Id"
          baseValue="ws_udl"
        />
        <OverrideField
          label="X-Client-Version"
          baseValue="2026.05.0"
        />

        <GroupHeader title="Query params" count={2} addable />
        <OverrideField
          label="trace"
          baseValue="false"
        />
        <OverrideField
          label="locale"
          baseValue="pl-PL"
        />

        <GroupHeader title="Body fields" count={1} addable />
        <OverrideField
          label="device.platform"
          baseValue="macos"
        />
      </div>

      {/* Footer */}
      <div style={{
        display: "flex", alignItems: "center", gap: 8,
        padding: "8px 14px",
        borderTop: "1px solid var(--border)",
        background: "var(--surface)",
        fontSize: 11, color: "var(--fg-muted)",
      }}>
        <span className="mono">env://dev</span>
        <span style={{ color: "var(--fg-faint)" }}>·</span>
        <span>saved 4 min ago to local SQLite</span>
        <div style={{ flex: 1 }} />
        <button className="ap-btn sm ghost">Reset all in dev</button>
        <button className="ap-btn sm">Export env</button>
      </div>
    </div>
  );
}

Object.assign(window, { EnvOverridesPanel });
