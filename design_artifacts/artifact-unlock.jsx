// Artifact 4: Master password unlock modal — centered over dimmed app shell

const { useState: useS4 } = React;

function UnlockModalShellBackdrop() {
  // Heavily blurred-out version of the shell, just to convey context.
  return (
    <div style={{
      position: "absolute", inset: 0,
      background: "var(--bg)",
      display: "flex", flexDirection: "column",
      filter: "blur(3px) saturate(0.8)",
      opacity: 0.55,
      pointerEvents: "none",
    }}>
      <div style={{ height: 36, background: "var(--surface)", borderBottom: "1px solid var(--border)" }}>
        <TrafficLights />
      </div>
      <div style={{ flex: 1, display: "flex" }}>
        <div style={{ width: 240, background: "var(--bg)", borderRight: "1px solid var(--border)" }}>
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} style={{ height: 26, margin: "6px 10px", background: "var(--surface-2)", borderRadius: 4, opacity: 1 - i * 0.06 }} />
          ))}
        </div>
        <div style={{ width: 280, background: "var(--surface)", borderRight: "1px solid var(--border)" }}>
          {Array.from({ length: 8 }).map((_, i) => (
            <div key={i} style={{ height: 36, margin: "6px 10px", background: "var(--surface-2)", borderRadius: 4 }} />
          ))}
        </div>
        <div style={{ flex: 1, background: "var(--bg)" }}>
          <div style={{ height: 50, borderBottom: "1px solid var(--border)" }} />
          <div style={{ height: 32, borderBottom: "1px solid var(--border)" }} />
          <div style={{ padding: 20 }}>
            {Array.from({ length: 14 }).map((_, i) => (
              <div key={i} style={{ height: 14, marginBottom: 8, background: "var(--surface-2)", borderRadius: 3, width: `${30 + (i*13)%55}%` }} />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function UnlockModal() {
  const [password, setPassword] = useS4("hunter2_master_4f");
  const [remember, setRemember] = useS4(true);
  const [show, setShow] = useS4(false);

  return (
    <div className="ap" style={{
      width: "100%", height: "100%",
      borderRadius: 10, overflow: "hidden",
      border: "1px solid var(--border-strong)",
      boxShadow: "0 30px 60px rgba(0,0,0,0.4)",
      position: "relative",
      background: "var(--bg)",
    }}>
      <UnlockModalShellBackdrop />

      {/* Dark wash */}
      <div style={{
        position: "absolute", inset: 0,
        background: "radial-gradient(circle at 50% 40%, rgba(0,0,0,0.55), rgba(0,0,0,0.85) 70%)",
        backdropFilter: "blur(2px)",
      }} />

      {/* Modal */}
      <div style={{
        position: "absolute", inset: 0,
        display: "flex", alignItems: "center", justifyContent: "center",
        padding: 40,
      }}>
        <div style={{
          width: 420,
          background: "var(--surface)",
          border: "1px solid var(--border-strong)",
          borderRadius: 12,
          boxShadow: "0 24px 60px rgba(0,0,0,0.55), 0 0 0 1px rgba(245,158,11,0.08), 0 0 40px rgba(245,158,11,0.06)",
          overflow: "hidden",
        }}>
          {/* Header strip */}
          <div style={{
            padding: "24px 24px 14px",
            display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 16,
          }}>
            <div style={{
              width: 38, height: 38, borderRadius: 9,
              background: "linear-gradient(180deg, rgba(245,158,11,0.18), rgba(245,158,11,0.06))",
              border: "1px solid rgba(245,158,11,0.30)",
              display: "flex", alignItems: "center", justifyContent: "center",
              color: "var(--accent)",
              boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04), 0 0 24px rgba(245,158,11,0.12)",
            }}>
              <Icon d="<rect x='5' y='11' width='14' height='10' rx='2'/><path d='M8 11V7a4 4 0 0 1 8 0v4'/>" size={18} />
            </div>
            <div>
              <div style={{ fontSize: 17, fontWeight: 600, color: "var(--fg)", letterSpacing: "-0.012em" }}>
                Unlock production environment
              </div>
              <div style={{ fontSize: 12.5, color: "var(--fg-muted)", marginTop: 6, lineHeight: 1.5, maxWidth: 360 }}>
                Secrets in <span className="mono" style={{ color: "var(--fg-dim)" }}>env://prod</span> are
                encrypted with your master password. Apiovnia will decrypt them in memory for this session and
                use them to sign requests. Nothing is sent off-device.
              </div>
            </div>
          </div>

          {/* Env summary card */}
          <div style={{
            margin: "0 24px",
            padding: "10px 12px",
            background: "var(--bg)",
            border: "1px solid var(--border)",
            borderRadius: 8,
            display: "flex", alignItems: "center", gap: 10,
            fontSize: 11.5,
          }}>
            <span style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--err)", flexShrink: 0 }} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ color: "var(--fg)", fontWeight: 500 }}>prod</div>
              <div className="mono" style={{ color: "var(--fg-faint)", fontSize: 10.5, marginTop: 2 }}>
                14 encrypted fields · last unlocked 2 days ago
              </div>
            </div>
            <span style={{
              fontSize: 10, color: "var(--accent)",
              background: "var(--accent-bg)", border: "1px solid var(--accent-bd)",
              padding: "2px 6px", borderRadius: 4,
              textTransform: "uppercase", letterSpacing: ".08em", fontWeight: 600,
            }}>locked</span>
          </div>

          {/* Password input */}
          <div style={{ padding: "16px 24px 6px" }}>
            <label style={{
              display: "block",
              fontSize: 10.5, fontWeight: 600, color: "var(--fg-muted)",
              textTransform: "uppercase", letterSpacing: ".08em", marginBottom: 6,
            }}>Master password</label>
            <div style={{
              display: "flex", alignItems: "center",
              height: 36,
              background: "var(--bg)",
              border: "1px solid var(--accent-bd)",
              borderRadius: 7,
              padding: "0 4px 0 10px",
              boxShadow: "0 0 0 3px rgba(245,158,11,0.10)",
            }}>
              <span style={{ color: "var(--accent)", marginRight: 8 }}>
                <Icon d="<rect x='5' y='11' width='14' height='10' rx='2'/><path d='M8 11V7a4 4 0 0 1 8 0v4'/>" size={14} />
              </span>
              <input
                className="mono"
                type={show ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••••••"
                style={{
                  flex: 1, height: "100%", border: 0, outline: "none",
                  background: "transparent", color: "var(--fg)",
                  fontSize: 13, letterSpacing: "0.04em",
                }}
              />
              <button
                onClick={() => setShow(s => !s)}
                className="ap-btn icon sm ghost"
                title={show ? "Hide" : "Reveal"}
                style={{ width: 28, height: 28 }}
              >
                <Icon d={show
                  ? "<path d='M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z'/><circle cx='12' cy='12' r='3'/>"
                  : "<path d='m3 3 18 18'/><path d='M10.5 5.2A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4'/><path d='M6.6 6.6A17 17 0 0 0 2 12s3.5 7 10 7c1.5 0 2.9-.3 4.1-.8'/><path d='M14.1 14.1a3 3 0 0 1-4.2-4.2'/>"
                } size={14} />
              </button>
            </div>
            <div style={{
              display: "flex", alignItems: "center", gap: 6,
              marginTop: 8, fontSize: 10.5, color: "var(--fg-faint)",
            }}>
              <Icon d="<path d='M12 9v4M12 17h.01'/><path d='M10.3 3.9 2.7 17.5A2 2 0 0 0 4.4 20.5h15.2a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z'/>" size={11} />
              <span>Three wrong attempts will throttle prod requests for 60s.</span>
            </div>
          </div>

          {/* Remember + key hint */}
          <div style={{ padding: "14px 24px 4px", display: "flex", alignItems: "center", gap: 8 }}>
            <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", flex: 1 }}>
              <div style={{
                width: 16, height: 16, borderRadius: 4,
                border: "1px solid " + (remember ? "var(--accent)" : "var(--border-strong)"),
                background: remember ? "var(--accent)" : "transparent",
                display: "flex", alignItems: "center", justifyContent: "center",
                color: "#1A1102",
              }}
              onClick={() => setRemember(r => !r)}>
                {remember && <Icon d="<path d='M20 6 9 17l-5-5'/>" size={11} strokeWidth={2.5} />}
              </div>
              <span style={{ fontSize: 12, color: "var(--fg-dim)" }}>Remember for this session</span>
              <span style={{ fontSize: 10.5, color: "var(--fg-faint)" }}>(until Apiovnia quits)</span>
            </label>
          </div>

          {/* Buttons */}
          <div style={{
            padding: "16px 24px 18px",
            display: "flex", alignItems: "center", gap: 8,
          }}>
            <button className="ap-btn ghost" style={{ height: 34, padding: "0 14px", fontSize: 12.5 }}>
              Cancel
            </button>
            <div style={{ flex: 1 }} />
            <span style={{ fontSize: 10.5, color: "var(--fg-faint)", marginRight: 6 }}>
              <span className="ap-kbd">esc</span> to cancel
            </span>
            <button className="ap-btn cta" style={{ height: 34, padding: "0 18px", fontSize: 12.5 }}>
              <Icon d="<rect x='5' y='11' width='14' height='10' rx='2'/><path d='M8 11V7a4 4 0 0 1 8 0'/>" size={13} strokeWidth={1.8} />
              <span>Unlock</span>
              <span className="ap-kbd" style={{
                background: "rgba(0,0,0,0.18)", borderColor: "rgba(0,0,0,0.25)",
                color: "rgba(26,17,2,0.7)",
              }}>↵</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { UnlockModal });
