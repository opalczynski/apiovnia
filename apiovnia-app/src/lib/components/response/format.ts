/**
 * Tiny display helpers used by the response viewer. Kept out of the
 * components so we can unit-test if the formatters ever grow non-trivial.
 */

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} kB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

export type StatusKind = "ok" | "warn" | "err";

export function statusKind(code: number): StatusKind {
  if (code >= 200 && code < 300) return "ok";
  if (code >= 300 && code < 500) return "warn";
  return "err";
}

export type PrettyLang = "json" | "html" | "xml" | "plain";

/** Map a content-type to a CodeMirror language. Strips parameters and is
 *  case-insensitive. Used for both response Pretty view and request body
 *  preview on the Request tab. */
export function langFromContentType(ct: string | null | undefined): PrettyLang {
  if (!ct) return "plain";
  const c = ct.toLowerCase();
  if (c === "application/json" || c.endsWith("+json")) return "json";
  if (c === "text/html") return "html";
  if (c === "application/xml" || c === "text/xml" || c.endsWith("+xml")) return "xml";
  return "plain";
}

import type { HeaderEntry } from "$lib/types/domain";

/** Find a header value case-insensitively. */
export function findHeader(headers: HeaderEntry[], name: string): string | null {
  const target = name.toLowerCase();
  const hit = headers.find((h) => h.name.toLowerCase() === target);
  return hit?.value ?? null;
}

/** Extract the bare content-type (no parameters), lowercased. */
export function contentTypeOf(headers: HeaderEntry[]): string | null {
  const v = findHeader(headers, "content-type");
  if (!v) return null;
  return v.split(";")[0]?.trim().toLowerCase() ?? null;
}
