// Artifact 3: Response viewer — pretty mode, focused

const { useState: useS3 } = React;

const RICH_RESPONSE = {
  ok: true,
  data: {
    user: {
      id: "usr_01JA48VX9DP7HZG3M2QK4N",
      email: "tomek@udl.dev",
      display_name: "Tomek",
      role: "owner",
      verified: true,
      mfa_enabled: false,
      avatar: null,
      created_at: "2024-11-02T08:14:22Z",
      last_login_at: "2026-05-15T07:42:11Z"
    },
    session: {
      access_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c3JfMDFKQTQ4Vlg5RFA3In0.r7K_x2Yh3pQwLm8nVf9CqB",
      refresh_token: "rt_01JA48VX9DP7HZG3M2QK4N",
      expires_in: 3600,
      scopes: ["read:user", "write:properties", "read:bookings", "admin:webhooks"]
    },
    workspace: {
      id: "ws_udl",
      name: "UDL",
      tier: "pro",
      seats_used: 1,
      seats_total: 1,
      features: { sso: false, audit_log: true, sandbox: true }
    }
  },
  meta: {
    request_id: "req_8fH2nQwR4kT3",
    server_time_ms: 142,
    rate_limit: { remaining: 4998, reset_at: "2026-05-15T08:00:00Z" }
  }
};

function ResponseViewer() {
  const [query, setQuery] = useS3("");
  const [searchOpen, setSearchOpen] = useS3(true);

  return (
    <div className="ap" style={{
      width: "100%", height: "100%",
      background: "var(--surface)",
      display: "flex", flexDirection: "column",
      borderRadius: 10, overflow: "hidden",
      border: "1px solid var(--border-strong)",
      boxShadow: "0 30px 60px rgba(0,0,0,0.4)",
    }}>
      {/* Header */}
      <div style={{
        display: "flex", alignItems: "center", gap: 10,
        padding: "10px 14px",
        borderBottom: "1px solid var(--border)",
        background: "var(--surface)",
      }}>
        <span className="ap-status ok">200 OK</span>
        <div style={{ display: "flex", gap: 16, fontSize: 11, fontFamily: "var(--mono)" }}>
          <span><span style={{ color: "var(--fg-faint)" }}>time </span><span style={{ color: "var(--fg-dim)" }}>142ms</span></span>
          <span><span style={{ color: "var(--fg-faint)" }}>size </span><span style={{ color: "var(--fg-dim)" }}>2.3kb</span></span>
          <span><span style={{ color: "var(--fg-faint)" }}>type </span><span style={{ color: "var(--fg-dim)" }}>application/json</span></span>
        </div>
        <div style={{ flex: 1 }} />
        <div className="ap-tabs" style={{ border: 0, padding: 0, gap: 0 }}>
          {["Pretty","Raw","Headers","Preview"].map((t, i) => (
            <button key={t} className={"ap-tab" + (i === 0 ? " active" : "")} style={{ height: 28 }}>
              <span>{t}</span>
              {t === "Headers" && <span className="count">12</span>}
            </button>
          ))}
        </div>
      </div>

      {/* Toolbar */}
      <div style={{
        display: "flex", alignItems: "center", gap: 8,
        padding: "8px 14px",
        borderBottom: "1px solid var(--border-soft)",
        background: "var(--surface)",
      }}>
        <div style={{ display: "flex", gap: 2, background: "var(--surface-2)", padding: 2, borderRadius: 6, border: "1px solid var(--border)" }}>
          <button className="ap-btn sm" style={{ height: 20, padding: "0 8px", fontSize: 10.5 }}>JSON</button>
          <button className="ap-btn sm ghost" style={{ height: 20, padding: "0 8px", fontSize: 10.5 }}>Tree</button>
          <button className="ap-btn sm ghost" style={{ height: 20, padding: "0 8px", fontSize: 10.5 }}>Schema</button>
        </div>
        <button className="ap-btn sm ghost">Collapse all</button>
        <button className="ap-btn sm ghost">Expand all</button>
        <div style={{ flex: 1 }} />
        {searchOpen && (
          <div style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "0 6px 0 8px", height: 24,
            background: "var(--bg)", border: "1px solid var(--border)", borderRadius: 6,
          }}>
            <span style={{ color: "var(--fg-muted)" }}>{IC.search}</span>
            <input
              autoFocus
              className="ap-input"
              placeholder="Find in response…"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              style={{ border: 0, background: "transparent", padding: 0, height: 22, width: 180, fontFamily: "var(--mono)", fontSize: 11.5 }}
            />
            <span className="mono" style={{ fontSize: 10.5, color: "var(--fg-faint)", fontVariantNumeric: "tabular-nums" }}>
              {query ? "1/3" : ""}
            </span>
            <button className="ap-btn icon sm ghost" style={{ width: 18, height: 18 }}>{IC.x}</button>
          </div>
        )}
        <button className="ap-btn sm ghost" title="Search (⌘F)">{IC.search}</button>
        <button className="ap-btn sm ghost">{IC.copy}<span>Copy</span></button>
        <button className="ap-btn sm ghost">Save as…</button>
      </div>

      {/* JSON body */}
      <div style={{ flex: 1, overflow: "auto", background: "var(--bg)", padding: "10px 0 14px" }}>
        <JsonView data={RICH_RESPONSE} searchQuery={query} />
      </div>

      {/* Footer */}
      <div style={{
        display: "flex", alignItems: "center", gap: 14,
        padding: "8px 14px",
        borderTop: "1px solid var(--border)",
        background: "var(--surface)",
        fontSize: 11,
        fontFamily: "var(--mono)",
        color: "var(--fg-muted)",
      }}>
        <span className="ap-status ok" style={{ padding: "2px 6px", fontSize: 10.5 }}>200</span>
        <span style={{ color: "var(--fg-dim)" }}>142ms</span>
        <span style={{ color: "var(--fg-faint)" }}>·</span>
        <span style={{ color: "var(--fg-dim)" }}>2.3kb</span>
        <span style={{ color: "var(--fg-faint)" }}>·</span>
        <span style={{ color: "var(--fg-dim)" }}>application/json</span>
        <span style={{ color: "var(--fg-faint)" }}>·</span>
        <span style={{ color: "var(--fg-dim)" }}>req_8fH2nQwR4kT3</span>
        <div style={{ flex: 1 }} />
        <span style={{ color: "var(--fg-faint)" }}>UTF-8</span>
        <span style={{ color: "var(--fg-faint)" }}>·</span>
        <span style={{ color: "var(--fg-faint)" }}>30 lines · 64 keys</span>
      </div>
    </div>
  );
}

Object.assign(window, { ResponseViewer, RICH_RESPONSE });
