<!--
  CodeMirrorEditor — thin Svelte wrapper around CodeMirror 6 with a dark
  theme matching our tokens. Supports `language: "json" | "plain"`, two-way
  `value` (bindable), and `readOnly`.

  We don't bring in @codemirror/theme-one-dark — we render JSON tokens with
  our own --j-* colors so the editor reads identically to the static JSON
  examples elsewhere in the app.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
  import { Annotation, Compartment, EditorState } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import {
    HighlightStyle,
    syntaxHighlighting,
    bracketMatching,
    indentOnInput,
    StreamLanguage,
    type StreamParser,
  } from "@codemirror/language";
  import { json, jsonParseLinter } from "@codemirror/lang-json";
  import { html } from "@codemirror/lang-html";
  import { xml } from "@codemirror/lang-xml";
  import { diagnosticCount, linter, lintGutter } from "@codemirror/lint";
  import { tags as t } from "@lezer/highlight";

  type Language = "json" | "html" | "xml" | "graphql" | "plain";

  type Props = {
    value: string;
    onChange: (next: string) => void;
    language?: Language;
    readOnly?: boolean;
    /** CSS height for the editor wrapper. */
    height?: string;
    /** Disable history (for short single-line cases). */
    noHistory?: boolean;
    /**
     * Enable language-aware validation. Currently only `language: "json"`
     * has a linter (parse errors flagged in the gutter); other languages
     * just enable the gutter so the UI doesn't shift when errors appear.
     */
    lint?: boolean;
    /** Notified whenever the diagnostic count changes (0 → clean). */
    onLintChange?: (count: number) => void;
  };

  let {
    value,
    onChange,
    language = "plain",
    readOnly = false,
    height = "100%",
    noHistory = false,
    lint = false,
    onLintChange,
  }: Props = $props();

  let hostEl: HTMLDivElement | undefined = $state();
  let view: EditorView | undefined;

  // Compartments let us swap config dynamically (lang + readOnly) without
  // destroying the view.
  const langComp = new Compartment();
  const readOnlyComp = new Compartment();
  const lintComp = new Compartment();

  // Annotation marker — transactions we dispatch ourselves to mirror an
  // external `value` change carry this, so the updateListener can skip
  // re-emitting (which would cause an echo loop and can drop edits when
  // it races with user input like paste).
  const External = Annotation.define<true>();

  const highlightStyle = HighlightStyle.define([
    { tag: t.string, color: "var(--j-string)" },
    { tag: t.number, color: "var(--j-number)" },
    { tag: t.bool, color: "var(--j-bool)" },
    { tag: t.null, color: "var(--j-null)" },
    { tag: t.propertyName, color: "var(--j-key)" },
    { tag: t.punctuation, color: "var(--j-punct)" },
    { tag: t.bracket, color: "var(--j-punct)" },
    { tag: t.keyword, color: "var(--accent)" },
    // GraphQL adds three tags the JSON/HTML/XML modes never emit.
    { tag: t.comment, color: "var(--fg-faint)", fontStyle: "italic" },
    { tag: t.variableName, color: "var(--j-key)" },
    { tag: t.meta, color: "var(--j-bool)" },
  ]);

  // GraphQL mode — a tiny hand-rolled StreamLanguage so we don't pull in
  // `cm6-graphql` + the `graphql` reference impl just for highlighting.
  // Schema-aware autocomplete (introspection) is a deliberate follow-up.
  const GQL_KEYWORDS = new Set([
    "query", "mutation", "subscription", "fragment", "on", "true", "false",
    "null", "type", "input", "interface", "union", "enum", "scalar",
    "schema", "directive", "extend", "implements",
  ]);

  type GqlState = { inBlockString: boolean };

  const graphqlParser: StreamParser<GqlState> = {
    name: "graphql",
    startState: () => ({ inBlockString: false }),
    token(stream, state) {
      // Resume an open `"""…"""` block string from a previous line.
      if (state.inBlockString) {
        while (!stream.eol()) {
          if (stream.match('"""')) {
            state.inBlockString = false;
            return "gqlString";
          }
          stream.next();
        }
        return "gqlString";
      }
      if (stream.eatSpace()) return null;
      const ch = stream.peek();
      if (ch == null) return null;

      if (ch === "#") {
        stream.skipToEnd();
        return "gqlComment";
      }
      if (stream.match('"""')) {
        while (!stream.eol()) {
          if (stream.match('"""')) return "gqlString";
          stream.next();
        }
        state.inBlockString = true;
        return "gqlString";
      }
      if (ch === '"') {
        stream.next();
        let escaped = false;
        let c: string | void;
        while ((c = stream.next()) != null) {
          if (c === '"' && !escaped) break;
          escaped = !escaped && c === "\\";
        }
        return "gqlString";
      }
      if (ch === "$") {
        stream.next();
        stream.eatWhile(/[\w]/);
        return "gqlVariable";
      }
      if (ch === "@") {
        stream.next();
        stream.eatWhile(/[\w]/);
        return "gqlDirective";
      }
      if (/[0-9]/.test(ch) || ch === "-") {
        stream.next();
        stream.eatWhile(/[0-9.eE+-]/);
        return "gqlNumber";
      }
      if (/[_A-Za-z]/.test(ch)) {
        stream.eatWhile(/[_A-Za-z0-9]/);
        return GQL_KEYWORDS.has(stream.current()) ? "gqlKeyword" : null;
      }
      if (/[{}()[\]:!=|&.,]/.test(ch)) {
        stream.next();
        return "gqlPunct";
      }
      stream.next();
      return null;
    },
    tokenTable: {
      gqlKeyword: t.keyword,
      gqlString: t.string,
      gqlComment: t.comment,
      gqlNumber: t.number,
      gqlVariable: t.variableName,
      gqlDirective: t.meta,
      gqlPunct: t.punctuation,
    },
  };

  const graphqlLang = StreamLanguage.define(graphqlParser);

  const baseTheme = EditorView.theme(
    {
      "&": {
        height: "100%",
        backgroundColor: "var(--bg)",
        color: "var(--fg)",
        fontSize: "12.5px",
      },
      ".cm-scroller": {
        fontFamily: "var(--mono)",
        lineHeight: "20px",
      },
      ".cm-content": {
        caretColor: "var(--accent)",
        padding: "10px 0",
      },
      ".cm-gutters": {
        backgroundColor: "var(--bg)",
        color: "var(--fg-faint)",
        border: "0",
        fontSize: "11px",
      },
      ".cm-activeLine": {
        backgroundColor: "color-mix(in srgb, var(--fg) 3%, transparent)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "transparent",
        color: "var(--fg-muted)",
      },
      ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection":
        {
          backgroundColor: "rgba(245,158,11,0.22)",
        },
      ".cm-cursor": {
        borderLeftColor: "var(--accent)",
      },
      ".cm-matchingBracket": {
        outline: "1px solid var(--accent-bd)",
      },

      // Lint marker, tooltip and inline-decoration styling lives in the
      // global stylesheet (app.css) — those rules need higher specificity
      // than what `EditorView.theme()` can produce, plus `!important` on a
      // couple of CodeMirror defaults we want to override.
    },
    { dark: true },
  );

  function langExtension(lang: Language) {
    switch (lang) {
      case "json":
        return json();
      case "html":
        return html();
      case "xml":
        return xml();
      case "graphql":
        return graphqlLang;
      case "plain":
      default:
        return [];
    }
  }

  function lintExtensions(enabled: boolean, lang: Language) {
    if (!enabled) return [];
    // JSON has a builtin parse-linter. HTML/XML/plain just get the gutter
    // so future linters slot in without a layout shift.
    //
    // `tooltipFilter: () => []` stops the inline-range hover tooltip so
    // the gutter dot's tooltip is the only one that fires (no double
    // popovers). Diagnostics still land in the state so the banner /
    // diagnosticCount keep working; the inline range mark itself is
    // hidden via CSS (app.css → .cm-lintRange*).
    if (lang === "json") {
      return [
        lintGutter(),
        linter(jsonParseLinter(), {
          delay: 250,
          tooltipFilter: () => [],
        }),
      ];
    }
    return [lintGutter()];
  }

  $effect(() => {
    const host = hostEl;
    if (!host) return;

    // Build the view exactly ONCE. The body reads `value` / `language` /
    // `lint` / `readOnly` / `noHistory` — wrapping it in `untrack` keeps
    // those reads from registering as effect dependencies, so a later
    // change to any of them does NOT re-run this effect (which would
    // destroy the live editor mid-edit and steal focus). Each of those
    // props has its own dedicated reconfigure effect below.
    untrack(() => {
      const state = EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          bracketMatching(),
          indentOnInput(),
          syntaxHighlighting(highlightStyle),
          baseTheme,
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          ...(noHistory ? [] : [history()]),
          langComp.of(langExtension(language)),
          lintComp.of(lintExtensions(lint, language)),
          readOnlyComp.of(EditorState.readOnly.of(readOnly)),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) {
              // Skip our own external-sync dispatches.
              if (!u.transactions.some((tr) => tr.annotation(External))) {
                onChange(u.state.doc.toString());
              }
            }
            // Diagnostics arrive on their own transactions (set by linter()).
            // Notify on every update so consumers don't miss the "now clean"
            // signal when the last error gets fixed.
            if (onLintChange) {
              onLintChange(diagnosticCount(u.state));
            }
          }),
        ],
      });

      view = new EditorView({ state, parent: host });
    });

    return () => {
      view?.destroy();
      view = undefined;
    };
  });

  // Push external `value` changes into the editor (e.g. when switching to a
  // different request) without echoing back to onChange.
  $effect(() => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== value) {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
        annotations: External.of(true),
      });
    }
  });

  $effect(() => {
    if (!view) return;
    view.dispatch({ effects: langComp.reconfigure(langExtension(language)) });
  });

  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: lintComp.reconfigure(lintExtensions(lint, language)),
    });
  });

  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: readOnlyComp.reconfigure(EditorState.readOnly.of(readOnly)),
    });
  });
</script>

<div bind:this={hostEl} class="cm-host" style="height: {height};"></div>

<style>
  .cm-host {
    width: 100%;
    background: var(--bg);
    overflow: hidden;
  }
  /* Reset default CodeMirror outlines — focus indicated by caret colour. */
  :global(.cm-editor) {
    outline: none !important;
  }
  :global(.cm-editor.cm-focused) {
    outline: none !important;
  }
</style>
