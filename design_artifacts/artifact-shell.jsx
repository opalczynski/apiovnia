// Artifact 1: Main shell — full 3-panel app view

const { useState: useS1 } = React;

const SAMPLE_RESPONSE = {
  ok: true,
  data: {
    access_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c3JfMDFKQTQ4Vlg5RFA3IiwiaWF0IjoxNzE1ODI4ODAwfQ.r7K_x2Yh3pQwLm8nVf9CqB",
    refresh_token: "rt_01JA48VX9DP7HZG3M2QK4N",
    expires_in: 3600,
    user: {
      id: "usr_01JA48VX9DP7",
      email: "tomek@udl.dev",
      display_name: "Tomek",
      role: "owner",
      created_at: "2024-11-02T08:14:22Z",
      verified: true
    },
    workspace: { id: "ws_udl", name: "UDL", tier: "pro" }
  },
  meta: { request_id: "req_8fH2nQ", server_time_ms: 142 }
};

function TitleBar() {
  return (
    <div style={{
      height: 36,
      display: "flex",
      alignItems: "center",
      borderBottom: "1px solid var(--border)",
      background: "var(--surface)",
      paddingRight: 10,
      flexShrink: 0,
    }}>
      <TrafficLights />
      <div style={{ flex: 1, display: "flex", justifyContent: "center" }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "4px 10px",
          fontSize: 11, color: "var(--fg-muted)",
          background: "var(--surface-2)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          minWidth: 320,
          justifyContent: "center",
        }}>
          <span style={{ color: "var(--fg-dim)" }}>UDL</span>
          <span style={{ color: "var(--fg-faint)" }}>/</span>
          <span style={{ color: "var(--fg-dim)" }}>Auth</span>
          <span style={{ color: "var(--fg-faint)" }}>/</span>
          <span style={{ color: "var(--fg)" }}>Login</span>
        </div>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <button className="ap-btn ghost sm" style={{ gap: 5 }}>
          <span style={{ opacity: .7 }}>{IC.search}</span>
          <span>Search</span>
          <span className="ap-kbd" style={{ marginLeft: 4 }}>⌘K</span>
        </button>
        <button className="ap-btn icon sm ghost" title="Settings">{IC.settings}</button>
      </div>
    </div>
  );
}

function LeftPanel({ width }) {
  const [activeProject, setActiveProject] = useS1("UDL");
  const [openCollections, setOpenCollections] = useS1({ Auth: true, Properties: false, Bookings: false });
  const projects = [
    { name: "UDL", count: 47, active: true },
    { name: "Resi4Rent", count: 23 },
    { name: "Personal API tests", count: 11 },
  ];
  const collections = [
    { name: "Auth", count: 4 },
    { name: "Properties", count: 12 },
    { name: "Bookings", count: 9 },
    { name: "Tenants", count: 7 },
    { name: "Webhooks", count: 5 },
  ];
  return (
    <div style={{ width, background: "var(--bg)", display: "flex", flexDirection: "column", minWidth: 0 }}>
      <div style={{ padding: "10px 8px 8px", display: "flex", alignItems: "center", gap: 6 }}>
        <div style={{
          flex: 1, display: "flex", alignItems: "center", gap: 6,
          padding: "0 8px", height: 26, borderRadius: 6,
          background: "var(--surface)", border: "1px solid var(--border)",
        }}>
          <span style={{ color: "var(--fg-muted)" }}>{IC.search}</span>
          <input className="ap-input" placeholder="Filter…"
            style={{ border: 0, background: "transparent", padding: 0, flex: 1, height: 22 }} />
          <span className="ap-kbd" style={{ opacity: .8 }}>⌘P</span>
        </div>
        <button className="ap-btn icon sm ghost" title="New project">{IC.plus}</button>
      </div>

      <div style={{ padding: "0 6px" }}>
        <div className="ap-sec">Projects</div>
        {projects.map((p) => (
          <div key={p.name}
               className={"ap-row" + (activeProject === p.name ? " active" : "")}
               onClick={() => setActiveProject(p.name)}>
            <span style={{ marginRight: 6, color: "var(--fg-muted)", opacity: activeProject === p.name ? 1 : .65 }}>
              {IC.folder}
            </span>
            <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.name}</span>
            <span style={{ color: "var(--fg-faint)", fontSize: 10, fontVariantNumeric: "tabular-nums" }}>{p.count}</span>
          </div>
        ))}
      </div>

      <div style={{ padding: "8px 6px 0", borderTop: "1px solid var(--border-soft)", marginTop: 8 }}>
        <div className="ap-sec" style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Collections · UDL</span>
          <span style={{ cursor: "pointer", color: "var(--fg-muted)" }}>{IC.plus}</span>
        </div>
        {collections.map((c) => {
          const open = openCollections[c.name];
          return (
            <React.Fragment key={c.name}>
              <div className={"ap-row" + (c.name === "Auth" ? " active" : "")}
                   onClick={() => setOpenCollections(s => ({ ...s, [c.name]: !s[c.name] }))}>
                <span style={{ marginRight: 4, color: "var(--fg-muted)", transform: open ? "rotate(90deg)" : "none", transition: "transform .15s" }}>
                  {IC.chevronR}
                </span>
                <span style={{ marginRight: 6, color: "var(--fg-muted)" }}>{IC.collection}</span>
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.name}</span>
                <span style={{ color: "var(--fg-faint)", fontSize: 10, fontVariantNumeric: "tabular-nums" }}>{c.count}</span>
              </div>
            </React.Fragment>
          );
        })}
      </div>

      <div style={{ flex: 1 }} />

      <div style={{
        padding: "8px 10px", borderTop: "1px solid var(--border)",
        display: "flex", alignItems: "center", gap: 8, color: "var(--fg-muted)", fontSize: 11,
      }}>
        <div style={{
          width: 20, height: 20, borderRadius: 4,
          background: "linear-gradient(135deg, #F59E0B, #B47208)",
          display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 10, fontWeight: 700, color: "#1A1102",
        }}>T</div>
        <div style={{ flex: 1, lineHeight: 1.2 }}>
          <div style={{ color: "var(--fg)", fontSize: 11.5 }}>Tomek</div>
          <div style={{ fontSize: 10, color: "var(--fg-faint)" }}>Local · SQLite</div>
        </div>
        <button className="ap-btn icon sm ghost">{IC.history}</button>
      </div>
    </div>
  );
}

function MidPanel({ width }) {
  const requests = [
    { method: "POST", name: "Login", subtitle: "/auth/login", active: true },
    { method: "POST", name: "Refresh token", subtitle: "/auth/refresh" },
    { method: "POST", name: "Logout", subtitle: "/auth/logout" },
    { method: "GET",  name: "Me", subtitle: "/auth/me" },
    { method: "POST", name: "Register", subtitle: "/auth/register" },
    { method: "POST", name: "Forgot password", subtitle: "/auth/password/forgot" },
    { method: "PATCH",name: "Change password", subtitle: "/auth/password/change" },
    { method: "DELETE", name: "Revoke session", subtitle: "/auth/sessions/:id" },
  ];
  return (
    <div style={{ width, background: "var(--surface)", display: "flex", flexDirection: "column", borderLeft: "1px solid var(--border)", minWidth: 0 }}>
      <div style={{ padding: "10px 8px 8px", display: "flex", alignItems: "center", gap: 6 }}>
        <div style={{
          flex: 1, display: "flex", alignItems: "center", gap: 6,
          padding: "0 8px", height: 26, borderRadius: 6,
          background: "var(--bg)", border: "1px solid var(--border)",
        }}>
          <span style={{ color: "var(--fg-muted)" }}>{IC.search}</span>
          <input className="ap-input" placeholder="Filter requests…"
            style={{ border: 0, background: "transparent", padding: 0, flex: 1, height: 22 }} />
        </div>
        <button className="ap-btn icon sm ghost" title="Filter by method">{IC.filter}</button>
        <button className="ap-btn icon sm ghost" title="New request">{IC.plus}</button>
      </div>
      <div style={{
        padding: "2px 10px 6px",
        fontSize: 10.5, color: "var(--fg-faint)",
        display: "flex", alignItems: "center", gap: 6,
        textTransform: "uppercase", letterSpacing: ".08em", fontWeight: 600,
      }}>
        <span>Auth</span>
        <span style={{ color: "var(--fg-faint)" }}>· 8 requests</span>
      </div>
      <div style={{ padding: "0 6px", overflow: "auto", flex: 1 }}>
        {requests.map((r, i) => (
          <div key={i} className={"ap-row" + (r.active ? " active" : "")}
               style={{ height: 36, alignItems: "center", padding: "0 8px", gap: 8 }}>
            <MethodBadge m={r.method} />
            <div style={{ flex: 1, minWidth: 0, lineHeight: 1.15 }}>
              <div style={{ fontSize: 12.5, color: "var(--fg)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.name}</div>
              <div className="mono" style={{ fontSize: 10.5, color: "var(--fg-faint)", marginTop: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {r.subtitle}
              </div>
            </div>
            {r.active ? (
              <span style={{ color: "var(--accent)" }}>{IC.chevronR}</span>
            ) : (
              <span style={{ color: "var(--fg-faint)", opacity: 0 }} className="row-chevron">{IC.chevronR}</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function RequestUrlBar() {
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 6,
      padding: "10px 12px",
      borderBottom: "1px solid var(--border)",
      background: "var(--surface)",
    }}>
      <button style={{
        display: "flex", alignItems: "center", gap: 6,
        height: 30, padding: "0 8px 0 8px",
        background: "var(--surface-2)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        cursor: "pointer",
      }}>
        <MethodBadge m="POST" />
        <span style={{ color: "var(--fg-muted)" }}>{IC.caret}</span>
      </button>
      <div style={{
        flex: 1, display: "flex", alignItems: "center", gap: 0,
        height: 30, padding: "0 8px",
        background: "var(--bg)",
        border: "1px solid var(--border)",
        borderRadius: 6,
      }}>
        <span className="mono" style={{ color: "var(--fg-faint)", fontSize: 12 }}>https://</span>
        <span className="mono" style={{ color: "var(--accent)", fontSize: 12 }}>{"{{base_url}}"}</span>
        <span className="mono" style={{ color: "var(--fg)", fontSize: 12 }}>/auth/login</span>
        <div style={{ flex: 1 }} />
        <span className="ap-kbd" style={{ opacity: .9 }}>⌘↵</span>
      </div>
      <button className="ap-btn cta" style={{ height: 30, padding: "0 16px" }}>
        {IC.send}
        <span>Send</span>
      </button>
      <button style={{
        display: "flex", alignItems: "center", gap: 6,
        height: 30, padding: "0 10px",
        background: "var(--surface-2)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        cursor: "pointer",
        color: "var(--fg)",
        fontSize: 12,
      }} title="Environment">
        <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--ok)" }} />
        <span>dev</span>
        <span style={{ color: "var(--fg-muted)" }}>{IC.caret}</span>
      </button>
    </div>
  );
}

function RequestTabs({ active = "Body" }) {
  const tabs = [
    { id: "Params",        count: 2 },
    { id: "Headers",       count: 3 },
    { id: "Body",          count: null, badge: "json" },
    { id: "Auth",          count: null, badge: "bearer" },
    { id: "Env Overrides", count: 1 },
    { id: "Tests",         count: 0, soon: true },
  ];
  return (
    <div className="ap-tabs">
      {tabs.map((t) => (
        <button key={t.id} className={"ap-tab" + (t.id === active ? " active" : "")}>
          <span>{t.id}</span>
          {t.count !== null && t.count !== undefined && t.count > 0 && <span className="count">{t.count}</span>}
          {t.badge && <span className="count mono" style={{ textTransform: "uppercase", letterSpacing: ".05em" }}>{t.badge}</span>}
          {t.soon && <span className="count" style={{ opacity: .6 }}>soon</span>}
        </button>
      ))}
    </div>
  );
}

function BodyPanel() {
  const json = `{
  "email": "tomek@udl.dev",
  "password": "{{master_password}}",
  "device": {
    "name": "Apiovnia/dev",
    "platform": "macos"
  }
}`;
  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
      <div style={{
        display: "flex", alignItems: "center", gap: 6,
        padding: "8px 12px",
        borderBottom: "1px solid var(--border-soft)",
      }}>
        <div style={{ display: "flex", gap: 2, background: "var(--surface-2)", borderRadius: 6, padding: 2, border: "1px solid var(--border)" }}>
          {["JSON", "Form", "Raw", "Binary", "GraphQL"].map((k, i) => (
            <button key={k} className="ap-btn sm ghost" style={{
              height: 20, padding: "0 8px", fontSize: 10.5,
              background: i === 0 ? "var(--bg)" : "transparent",
              color: i === 0 ? "var(--fg)" : "var(--fg-muted)",
              border: i === 0 ? "1px solid var(--border-strong)" : "1px solid transparent",
            }}>{k}</button>
          ))}
        </div>
        <div style={{ flex: 1 }} />
        <button className="ap-btn sm ghost">Beautify</button>
        <button className="ap-btn sm ghost">Vars</button>
      </div>
      <div style={{ flex: 1, padding: "10px 0 10px 8px", overflow: "auto", background: "var(--bg)", display: "flex" }}>
        <div className="mono" style={{
          color: "var(--fg-faint)", fontSize: 11.5, lineHeight: "20px",
          textAlign: "right", paddingRight: 10, userSelect: "none", minWidth: 30,
        }}>
          {json.split("\n").map((_, i) => <div key={i}>{i + 1}</div>)}
        </div>
        <pre className="mono" style={{
          margin: 0, fontSize: 12.5, lineHeight: "20px", color: "var(--fg)",
          flex: 1, whiteSpace: "pre",
        }}>
{`{
  `}<span className="j-key">"email"</span>{`: `}<span className="j-string">"tomek@udl.dev"</span>{`,
  `}<span className="j-key">"password"</span>{`: `}<span style={{ color: "var(--accent)" }}>"{`{{master_password}}`}"</span>{`,
  `}<span className="j-key">"device"</span>{`: {
    `}<span className="j-key">"name"</span>{`: `}<span className="j-string">"Apiovnia/dev"</span>{`,
    `}<span className="j-key">"platform"</span>{`: `}<span className="j-string">"macos"</span>{`
  }
}`}
        </pre>
      </div>
    </div>
  );
}

function ResponseHeader() {
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 10,
      padding: "8px 12px",
      borderBottom: "1px solid var(--border)",
      background: "var(--surface)",
    }}>
      <span className="ap-status ok">200 OK</span>
      <div style={{ display: "flex", gap: 14, color: "var(--fg-muted)", fontSize: 11, fontFamily: "var(--mono)" }}>
        <span><span style={{ color: "var(--fg-faint)" }}>time </span><span style={{ color: "var(--fg-dim)" }}>142ms</span></span>
        <span><span style={{ color: "var(--fg-faint)" }}>size </span><span style={{ color: "var(--fg-dim)" }}>2.3kb</span></span>
        <span style={{ color: "var(--fg-faint)" }}>application/json</span>
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
      <button className="ap-btn sm ghost" title="Copy response">{IC.copy}</button>
    </div>
  );
}

function ResponseBody() {
  return (
    <div style={{ flex: 1, overflow: "auto", background: "var(--bg)", padding: "8px 0" }}>
      <JsonView data={SAMPLE_RESPONSE} initialCollapsed={["$.data.workspace"]} />
    </div>
  );
}

function RightPanel() {
  return (
    <div style={{
      flex: 1, minWidth: 0,
      background: "var(--bg)",
      borderLeft: "1px solid var(--border)",
      display: "flex", flexDirection: "column",
    }}>
      <RequestUrlBar />
      <RequestTabs active="Body" />
      <BodyPanel />

      {/* Horizontal splitter */}
      <div style={{
        height: 4,
        background: "var(--border)",
        position: "relative",
        cursor: "row-resize",
      }}>
        <div style={{
          position: "absolute", left: "50%", top: 0, transform: "translateX(-50%)",
          width: 28, height: 4, background: "var(--border-strong)", borderRadius: 2,
        }} />
      </div>

      <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0, background: "var(--surface)" }}>
        <ResponseHeader />
        <ResponseBody />
      </div>
    </div>
  );
}

function MainShell() {
  return (
    <div className="ap" style={{
      width: "100%", height: "100%",
      display: "flex", flexDirection: "column",
      background: "var(--bg)",
      borderRadius: 10, overflow: "hidden",
      border: "1px solid var(--border-strong)",
      boxShadow: "0 30px 60px rgba(0,0,0,0.4)",
    }}>
      <TitleBar />
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <LeftPanel width={240} />
        <div className="ap-resizer" />
        <MidPanel width={280} />
        <div className="ap-resizer" />
        <RightPanel />
      </div>
    </div>
  );
}

Object.assign(window, { MainShell, SAMPLE_RESPONSE });
