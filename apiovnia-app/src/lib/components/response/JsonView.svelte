<!--
  JsonView — collapsible JSON renderer with syntax highlight, ⌘F search +
  next/prev navigation, hover-copy per value, and expand/collapse-all.

  We render a flat list of "lines" so search/navigation can index by line
  number without recursing the tree per keystroke.

  The toolbar (search + counter + nav + expand/collapse all) is built in
  so the consumer just passes `data` (parsed JSON value) and we own the
  state. ⌘F is captured globally while this component is mounted so the
  shortcut works no matter where focus is — Esc clears the query.
-->
<script lang="ts">
  import Icon from "$lib/components/Icon.svelte";
  import { IC } from "$lib/components/icons";
  import { onMount } from "svelte";

  type Props = {
    /** Parsed JSON value to render. Any JS value works — primitives at the
     *  root degrade to a single line. */
    data: unknown;
  };

  const { data }: Props = $props();

  // ---------------------------------------------------------------------
  // Flatten: walk the tree once and produce a list of render-ready lines.
  // Folded nodes show a `…N items / N keys` chip and skip their children.
  // ---------------------------------------------------------------------

  type Part =
    | { t: "ws" | "punct" | "key" | "string" | "number" | "bool" | "null"; v: string }
    | { t: "fold"; v: string; path: string };

  type Line = {
    no: number;
    depth: number;
    parts: Part[];
    path: string;
    foldable?: boolean;
    isOpen?: boolean;
    kind?: "object" | "array";
    /** Primitive lines expose hover-copy. */
    copyable?: boolean;
    copyValue?: string;
  };

  const INDENT = 2;
  const MAX_LINES = 8000; // safety net for pathological responses

  function flatten(value: unknown, collapsedSet: Set<string>): Line[] {
    const out: Line[] = [];
    let no = 0;
    let truncated = false;

    const push = (line: Omit<Line, "no">) => {
      if (out.length >= MAX_LINES) {
        truncated = true;
        return;
      }
      out.push({ no: ++no, ...line });
    };

    const punct = (v: string): Part => ({ t: "punct", v });

    function walk(val: unknown, depth: number, key: string | undefined, path: string, isLast: boolean) {
      const ws: Part = { t: "ws", v: " ".repeat(depth * INDENT) };
      const keyParts: Part[] = key !== undefined
        ? [{ t: "key", v: JSON.stringify(key) }, punct(": ")]
        : [];

      if (Array.isArray(val)) {
        const folded = collapsedSet.has(path);
        if (val.length === 0) {
          push({ depth, parts: [ws, ...keyParts, punct(isLast ? "[]" : "[],")], path });
          return;
        }
        push({
          depth,
          parts: [
            ws,
            ...keyParts,
            punct("["),
            ...(folded
              ? [{ t: "fold" as const, v: ` ${val.length} items `, path }, punct(isLast ? "]" : "],")]
              : []),
          ],
          path,
          foldable: true,
          isOpen: !folded,
          kind: "array",
        });
        if (!folded) {
          val.forEach((v, i) =>
            walk(v, depth + 1, undefined, `${path}[${i}]`, i === val.length - 1),
          );
          push({
            depth,
            parts: [ws, punct(isLast ? "]" : "],")],
            path: `${path}.__close__`,
          });
        }
      } else if (val && typeof val === "object") {
        const entries = Object.entries(val as Record<string, unknown>);
        const folded = collapsedSet.has(path);
        if (entries.length === 0) {
          push({ depth, parts: [ws, ...keyParts, punct(isLast ? "{}" : "{},")], path });
          return;
        }
        push({
          depth,
          parts: [
            ws,
            ...keyParts,
            punct("{"),
            ...(folded
              ? [{ t: "fold" as const, v: ` ${entries.length} keys `, path }, punct(isLast ? "}" : "},")]
              : []),
          ],
          path,
          foldable: true,
          isOpen: !folded,
          kind: "object",
        });
        if (!folded) {
          entries.forEach(([k, v], i) =>
            walk(v, depth + 1, k, `${path}.${k}`, i === entries.length - 1),
          );
          push({
            depth,
            parts: [ws, punct(isLast ? "}" : "},")],
            path: `${path}.__close__`,
          });
        }
      } else {
        let part: Part;
        let copyValue: string;
        if (typeof val === "string") {
          part = { t: "string", v: JSON.stringify(val) };
          copyValue = val;
        } else if (typeof val === "number") {
          part = { t: "number", v: String(val) };
          copyValue = String(val);
        } else if (typeof val === "boolean") {
          part = { t: "bool", v: String(val) };
          copyValue = String(val);
        } else if (val === null) {
          part = { t: "null", v: "null" };
          copyValue = "null";
        } else {
          // undefined / symbol / function — shouldn't appear in real JSON
          // but be safe.
          const s = String(val);
          part = { t: "string", v: JSON.stringify(s) };
          copyValue = s;
        }
        push({
          depth,
          parts: [ws, ...keyParts, part, ...(isLast ? [] : [punct(",")])],
          path,
          copyable: true,
          copyValue,
        });
      }
    }

    walk(value, 0, undefined, "$", true);
    return truncated
      ? [
          ...out,
          {
            no: out.length + 1,
            depth: 0,
            parts: [
              { t: "punct", v: `… ${MAX_LINES.toLocaleString()}+ lines — viewer truncated. See Raw tab for the full body.` },
            ],
            path: "$.__truncated__",
          },
        ]
      : out;
  }

  // ---------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------

  /** Set of paths that are currently collapsed. */
  let collapsed = $state(new Set<string>());

  /**
   * Bumped whenever the user clicks "Expand all" / "Collapse all". Lets
   * the derived `lines` recompute even though `collapsed` is the same
   * Set instance (Svelte's $state Set tracking is shallow).
   */
  let collapsedVersion = $state(0);

  let searchQuery = $state("");
  let activeMatch = $state(0);
  let searchInput: HTMLInputElement | undefined = $state();
  let container: HTMLDivElement | undefined = $state();
  /** Bumped each time the user hits the Copy button — drives the toast. */
  let lastCopied = $state<{ value: string; at: number } | null>(null);

  // ---------------------------------------------------------------------
  // Derived
  // ---------------------------------------------------------------------

  const lines = $derived.by(() => {
    // Touch `collapsedVersion` so a recompute kicks in after expand/
    // collapse-all even when the Set identity hasn't changed.
    void collapsedVersion;
    return flatten(data, collapsed);
  });

  /** Indices into `lines` where the search query matches (case-insensitive). */
  const matchIndices = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return [] as number[];
    const out: number[] = [];
    for (let i = 0; i < lines.length; i++) {
      const flat = lines[i].parts.map((p) => p.v).join("").toLowerCase();
      if (flat.includes(q)) out.push(i);
    }
    return out;
  });

  // Keep activeMatch within range as results change.
  $effect(() => {
    if (matchIndices.length === 0) {
      activeMatch = 0;
    } else if (activeMatch >= matchIndices.length) {
      activeMatch = 0;
    }
  });

  // Scroll the active match into view when it moves.
  $effect(() => {
    if (matchIndices.length === 0 || !container) return;
    const line = matchIndices[activeMatch];
    const el = container.querySelector<HTMLElement>(`[data-line="${line}"]`);
    if (el) el.scrollIntoView({ block: "center", behavior: "smooth" });
  });

  // ---------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------

  function toggle(path: string) {
    if (collapsed.has(path)) collapsed.delete(path);
    else collapsed.add(path);
    collapsedVersion++;
  }

  function expandAll() {
    collapsed.clear();
    collapsedVersion++;
  }

  function collapseAll() {
    // Collapse every foldable path one level deep — keeps the root expanded
    // so the user sees something. Walk the data once to discover them.
    const next = new Set<string>();
    function visit(val: unknown, path: string, depth: number) {
      if (Array.isArray(val)) {
        if (depth > 0 && val.length > 0) next.add(path);
        val.forEach((v, i) => visit(v, `${path}[${i}]`, depth + 1));
      } else if (val && typeof val === "object") {
        if (depth > 0 && Object.keys(val).length > 0) next.add(path);
        Object.entries(val as Record<string, unknown>).forEach(([k, v]) =>
          visit(v, `${path}.${k}`, depth + 1),
        );
      }
    }
    visit(data, "$", 0);
    collapsed = next;
    collapsedVersion++;
  }

  function focusSearch() {
    if (searchInput) {
      searchInput.focus();
      searchInput.select();
    }
  }

  function next() {
    if (matchIndices.length === 0) return;
    activeMatch = (activeMatch + 1) % matchIndices.length;
  }
  function prev() {
    if (matchIndices.length === 0) return;
    activeMatch = (activeMatch - 1 + matchIndices.length) % matchIndices.length;
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) prev();
      else next();
    } else if (e.key === "Escape") {
      e.preventDefault();
      searchQuery = "";
      searchInput?.blur();
    }
  }

  async function copyValue(v: string) {
    try {
      await navigator.clipboard.writeText(v);
      lastCopied = { value: v, at: Date.now() };
      setTimeout(() => {
        if (lastCopied && Date.now() - lastCopied.at >= 1200) lastCopied = null;
      }, 1300);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.warn("[apiovnia] clipboard write failed", e);
    }
  }

  // ---------------------------------------------------------------------
  // Global ⌘F / Ctrl+F — capture while the viewer is mounted.
  // ---------------------------------------------------------------------

  onMount(() => {
    function handler(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        focusSearch();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  // ---------------------------------------------------------------------
  // Render helper — split a part value around the search query so we can
  // wrap matches in <mark>. Returns an array of {text, isMatch}.
  // ---------------------------------------------------------------------
  function highlight(text: string, q: string): { text: string; isMatch: boolean }[] {
    if (!q) return [{ text, isMatch: false }];
    const lower = text.toLowerCase();
    const out: { text: string; isMatch: boolean }[] = [];
    let i = 0;
    while (i < text.length) {
      const idx = lower.indexOf(q, i);
      if (idx === -1) {
        out.push({ text: text.slice(i), isMatch: false });
        break;
      }
      if (idx > i) out.push({ text: text.slice(i, idx), isMatch: false });
      out.push({ text: text.slice(idx, idx + q.length), isMatch: true });
      i = idx + q.length;
    }
    return out;
  }

  function partClass(t: Part["t"]): string {
    switch (t) {
      case "key": return "j-key";
      case "string": return "j-string";
      case "number": return "j-number";
      case "bool": return "j-bool";
      case "null": return "j-null";
      case "punct": return "j-punct";
      case "ws": return "j-ws";
      case "fold": return "";
    }
  }
</script>

<div class="root">
  <div class="toolbar mono">
    <div class="search">
      <Icon d={IC.search} size={12} />
      <input
        bind:this={searchInput}
        class="ap-input search-input"
        type="text"
        placeholder="Search response… (⌘F)"
        bind:value={searchQuery}
        onkeydown={onSearchKey}
      />
      {#if searchQuery}
        <span class="counter">
          {matchIndices.length === 0 ? "0" : `${activeMatch + 1}/${matchIndices.length}`}
        </span>
        <button class="ap-btn icon sm ghost" title="Previous match (Shift+Enter)" onclick={prev} disabled={matchIndices.length === 0}>
          <Icon d="<polyline points='15 18 9 12 15 6' />" />
        </button>
        <button class="ap-btn icon sm ghost" title="Next match (Enter)" onclick={next} disabled={matchIndices.length === 0}>
          <Icon d="<polyline points='9 18 15 12 9 6' />" />
        </button>
        <button class="ap-btn icon sm ghost" title="Clear" onclick={() => { searchQuery = ''; searchInput?.focus(); }}>
          <Icon d={IC.x} />
        </button>
      {/if}
    </div>

    <span class="grow"></span>

    <button class="ap-btn sm ghost" onclick={expandAll} title="Expand all nodes">Expand all</button>
    <button class="ap-btn sm ghost" onclick={collapseAll} title="Collapse all nested nodes">Collapse all</button>
  </div>

  <div class="jv-scroll" bind:this={container}>
    <div class="jv">
      {#each lines as ln, i (ln.no)}
        {@const isMatch = matchIndices.includes(i)}
        {@const isActive = isMatch && matchIndices[activeMatch] === i}
        <div
          class="jv-line"
          class:match={isMatch}
          class:match-active={isActive}
          data-line={i}
        >
          <span class="jv-no">{ln.no}</span>
          <span class="jv-gutter">
            {#if ln.foldable}
              <button class="jv-fold-btn" onclick={() => toggle(ln.path)} aria-label="Toggle node">
                <svg width="10" height="10" viewBox="0 0 10 10" style:transform={ln.isOpen ? "rotate(90deg)" : "rotate(0deg)"} style:transition="transform .12s">
                  <path d="M3 2 L7 5 L3 8 Z" fill="currentColor" />
                </svg>
              </button>
            {/if}
          </span>
          <span class="jv-content">
            {#each ln.parts as part, pi (pi)}
              {#if part.t === "fold"}
                <button
                  type="button"
                  class="jv-foldlabel"
                  onclick={() => toggle(part.path)}
                  title="Expand"
                >…{part.v}</button>
              {:else}
                {@const segments = highlight(part.v, searchQuery.trim().toLowerCase())}
                <span class={partClass(part.t)}>
                  {#each segments as seg, si (si)}
                    {#if seg.isMatch}<mark>{seg.text}</mark>{:else}{seg.text}{/if}
                  {/each}
                </span>
              {/if}
            {/each}
          </span>
          {#if ln.copyable && ln.copyValue !== undefined}
            <button
              class="jv-copy"
              title="Copy value"
              onclick={(e) => {
                e.stopPropagation();
                copyValue(ln.copyValue!);
              }}
            >
              <Icon d={IC.copy} size={11} />
            </button>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  {#if lastCopied}
    <div class="toast mono" role="status">Copied</div>
  {/if}
</div>

<style>
  .root {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
    position: relative;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-soft);
    background: var(--surface);
    flex-shrink: 0;
  }
  .search {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    max-width: 420px;
    /* Container colour drives the search icon next to the input. */
    color: var(--fg-muted);
  }
  .search-input {
    flex: 1;
    min-width: 0;
    background: var(--bg);
  }
  .counter {
    font-size: 10.5px;
    color: var(--fg-muted);
    padding: 0 4px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .grow {
    flex: 1;
  }
  .jv-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 4px 0;
  }
  .toast {
    position: absolute;
    bottom: 10px;
    right: 10px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    color: var(--fg);
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 11px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
    animation: fade-in 0.12s ease-out;
    pointer-events: none;
  }
  @keyframes fade-in {
    from { opacity: 0; transform: translateY(2px); }
    to   { opacity: 1; transform: translateY(0); }
  }
</style>
