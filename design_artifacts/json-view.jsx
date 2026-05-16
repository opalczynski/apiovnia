// JsonView — collapsible, syntax-highlighted JSON with hover-copy per line.
// Renders a flat list of lines for fast keyboard navigation and stable copy.

const { useState, useMemo, useCallback, useRef, useEffect } = React;

function flattenJson(value, opts = {}) {
  const { collapsed = new Set(), indent = 2 } = opts;
  const lines = [];
  let lineNo = 0;

  function pushLine(depth, parts, path, meta = {}) {
    lines.push({ no: ++lineNo, depth, parts, path, ...meta });
  }

  function walk(val, depth, key, path, isLast) {
    const indentStr = " ".repeat(depth * indent);
    const keyPart = key !== undefined ? { t: "key", v: JSON.stringify(key) } : null;
    const punct = (v) => ({ t: "punct", v });

    if (Array.isArray(val)) {
      const collapsedHere = collapsed.has(path);
      if (val.length === 0) {
        pushLine(depth, [
          { t: "ws", v: indentStr },
          ...(keyPart ? [keyPart, punct(": ")] : []),
          punct(isLast ? "[]" : "[],"),
        ], path);
        return;
      }
      pushLine(depth, [
        { t: "ws", v: indentStr },
        ...(keyPart ? [keyPart, punct(": ")] : []),
        punct("["),
        ...(collapsedHere ? [{ t: "fold", v: ` ${val.length} items `, path }, punct(isLast ? "]" : "],")] : []),
      ], path, { foldable: true, isOpen: !collapsedHere, kind: "array" });
      if (!collapsedHere) {
        val.forEach((v, i) => walk(v, depth + 1, undefined, path + "[" + i + "]", i === val.length - 1));
        pushLine(depth, [
          { t: "ws", v: indentStr },
          punct(isLast ? "]" : "],"),
        ], path + ".__close__");
      }
    } else if (val && typeof val === "object") {
      const entries = Object.entries(val);
      const collapsedHere = collapsed.has(path);
      if (entries.length === 0) {
        pushLine(depth, [
          { t: "ws", v: indentStr },
          ...(keyPart ? [keyPart, punct(": ")] : []),
          punct(isLast ? "{}" : "{},"),
        ], path);
        return;
      }
      pushLine(depth, [
        { t: "ws", v: indentStr },
        ...(keyPart ? [keyPart, punct(": ")] : []),
        punct("{"),
        ...(collapsedHere ? [{ t: "fold", v: ` ${entries.length} keys `, path }, punct(isLast ? "}" : "},")] : []),
      ], path, { foldable: true, isOpen: !collapsedHere, kind: "object" });
      if (!collapsedHere) {
        entries.forEach(([k, v], i) => walk(v, depth + 1, k, path + "." + k, i === entries.length - 1));
        pushLine(depth, [
          { t: "ws", v: indentStr },
          punct(isLast ? "}" : "},"),
        ], path + ".__close__");
      }
    } else {
      let part;
      if (typeof val === "string") part = { t: "string", v: JSON.stringify(val) };
      else if (typeof val === "number") part = { t: "number", v: String(val) };
      else if (typeof val === "boolean") part = { t: "bool", v: String(val) };
      else if (val === null) part = { t: "null", v: "null" };
      else part = { t: "string", v: JSON.stringify(val) };
      pushLine(depth, [
        { t: "ws", v: indentStr },
        ...(keyPart ? [keyPart, punct(": ")] : []),
        part,
        ...(isLast ? [] : [punct(",")]),
      ], path, { copyable: true });
    }
  }
  walk(value, 0, undefined, "$", true);
  return lines;
}

const partClass = (t) => ({
  key: "j-key", string: "j-string", number: "j-number", bool: "j-bool",
  null: "j-null", punct: "j-punct", ws: "j-ws", fold: "j-fold",
}[t] || "");

function JsonView({ data, searchQuery = "", showLineNumbers = true, initialCollapsed = [] }) {
  const [collapsed, setCollapsed] = useState(() => new Set(initialCollapsed));
  const lines = useMemo(() => flattenJson(data, { collapsed }), [data, collapsed]);

  const toggle = useCallback((path) => {
    setCollapsed((c) => {
      const n = new Set(c);
      n.has(path) ? n.delete(path) : n.add(path);
      return n;
    });
  }, []);

  const q = searchQuery.trim().toLowerCase();

  return (
    <div className="jv">
      {lines.map((ln) => {
        const flatText = ln.parts.map(p => p.v).join("");
        const matches = q && flatText.toLowerCase().includes(q);
        return (
          <div key={ln.no} className={"jv-line" + (matches ? " match" : "")}>
            {showLineNumbers && <span className="jv-no">{ln.no}</span>}
            <span className="jv-gutter">
              {ln.foldable && (
                <button className="jv-fold-btn" onClick={() => toggle(ln.path)} aria-label="toggle">
                  <svg width="10" height="10" viewBox="0 0 10 10" style={{
                    transform: ln.isOpen ? "rotate(90deg)" : "rotate(0deg)",
                    transition: "transform .12s",
                  }}>
                    <path d="M3 2 L7 5 L3 8 Z" fill="currentColor" />
                  </svg>
                </button>
              )}
            </span>
            <span className="jv-content">
              {ln.parts.map((p, i) => {
                if (p.t === "fold") {
                  return <span key={i} className="jv-foldlabel" onClick={() => toggle(p.path)}>…{p.v}</span>;
                }
                let v = p.v;
                if (matches && q) {
                  const idx = v.toLowerCase().indexOf(q);
                  if (idx >= 0) {
                    return (
                      <span key={i} className={partClass(p.t)}>
                        {v.slice(0, idx)}
                        <mark>{v.slice(idx, idx + q.length)}</mark>
                        {v.slice(idx + q.length)}
                      </span>
                    );
                  }
                }
                return <span key={i} className={partClass(p.t)}>{v}</span>;
              })}
            </span>
            {ln.copyable && (
              <button className="jv-copy" title="Copy value" onClick={(e) => {
                e.stopPropagation();
                const val = ln.parts.find(p => ["string","number","bool","null"].includes(p.t));
                if (val) navigator.clipboard?.writeText(val.v.replace(/^"|"$/g, ""));
              }}>
                {IC.copy}
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}

Object.assign(window, { JsonView, flattenJson });
