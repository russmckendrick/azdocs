/**
 * Shared display formatting.
 *
 * Six `Intl.DateTimeFormat` wrappers lived across five components, two of them
 * the same option bag under different names, and `id.split("/").at(-1)` appeared
 * in six places. Formatters are also memoised here: constructing an
 * `Intl.DateTimeFormat` is not cheap and these run per row.
 */

import { createElement, Fragment, type ReactNode } from "react";

import { labels } from "./labels";

const memo = new Map<string, Intl.DateTimeFormat>();

function formatter(options: Intl.DateTimeFormatOptions) {
  const key = JSON.stringify(options);
  let existing = memo.get(key);
  if (!existing) {
    existing = new Intl.DateTimeFormat(undefined, options);
    memo.set(key, existing);
  }
  return existing;
}

function parse(value: string | null | undefined) {
  if (!value) return undefined;
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? undefined : date;
}

/** `04 Mar` — for axis ticks and dense strips where the year is implied. */
export function dayMonth(value: string | null | undefined) {
  const date = parse(value);
  return date ? formatter({ day: "2-digit", month: "short" }).format(date) : "";
}

/** `04 Mar 2026` — a date the reader may need to quote. */
export function dayMonthYear(value: string | null | undefined) {
  const date = parse(value);
  return date
    ? formatter({ day: "2-digit", month: "short", year: "numeric" }).format(date)
    : "";
}

/** `04 Mar, 14:32` — the masthead's snapshot stamp. */
export function dayMonthTime(value: string | null | undefined) {
  const date = parse(value);
  return date
    ? formatter({
        day: "2-digit",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date)
    : "";
}

/** `4 Mar 2026, 14:32` — a full stamp for history rows. */
export function dateTime(value: string | null | undefined) {
  const date = parse(value);
  return date ? formatter({ dateStyle: "medium", timeStyle: "short" }).format(date) : "";
}

/**
 * `4 Mar 2026, 14:32:07`, or `undefined` when the value is not a date.
 *
 * The adaptive record view uses the `undefined` to decide whether a string
 * field is a timestamp worth rendering as one, so this one must not fall back
 * to an empty string.
 */
export function preciseDateTime(value: string | null | undefined) {
  const date = parse(value);
  return date ? formatter({ dateStyle: "medium", timeStyle: "medium" }).format(date) : undefined;
}

/**
 * The last segment of an ARM id — the resource's own name.
 *
 * Falls back to the whole id rather than an empty string: a trailing slash or a
 * value that is not an ARM id at all should still show the reader something.
 */
export function resourceName(id: string | null | undefined) {
  if (!id) return "";
  return id.split("/").filter(Boolean).at(-1) ?? id;
}

/**
 * A message to show the reader for a thrown value.
 *
 * `catch` gives `unknown`; this narrows it in one place rather than each call
 * site inventing its own `instanceof Error` check. `fallback` covers the cases
 * where `String(error)` would surface something like "[object Object]", and
 * defaults to the labels' generic message.
 */
export function errorMessage(error: unknown, fallback = labels().desktop.errors.generic) {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error) return error;
  return fallback;
}

export type Vars = Record<string, string | number>;

/**
 * `{name}` substitution, the twin of Rust's `labels::fill`.
 *
 * An unknown placeholder is left in the output so a typo in an override file
 * shows on screen rather than vanishing; values are never re-scanned, so a
 * resource name containing braces is safe.
 */
export function fill(template: string, vars: Vars) {
  return template.replaceAll(/\{([a-z_][a-z0-9_]*)\}/g, (whole, name: string) =>
    name in vars ? String(vars[name]) : whole,
  );
}

/**
 * `fill` for templates whose values are elements — a `<span className="mono">`
 * inside a sentence. Text between placeholders becomes strings; each
 * placeholder becomes the node registered under its name, or is left as
 * text when nothing is. The twin of the print document's run splitter.
 */
export function fillNodes(template: string, nodes: Record<string, ReactNode>): ReactNode[] {
  const out: ReactNode[] = [];
  const pattern = /\{([a-z_][a-z0-9_]*)\}/g;
  let last = 0;
  for (const match of template.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > last) out.push(template.slice(last, index));
    const name = match[1];
    out.push(
      name in nodes ? createElement(Fragment, { key: `${name}-${index}` }, nodes[name]) : match[0],
    );
    last = index + match[0].length;
  }
  if (last < template.length) out.push(template.slice(last));
  return out;
}

export type PluralForms = { one: string; other: string };

/** The form for `count` (`one` for exactly one), with `{count}` and `vars` filled. */
export function plural(forms: PluralForms, count: number, vars: Vars = {}) {
  return fill(count === 1 ? forms.one : forms.other, { count, ...vars });
}

/** First letter upper-cased, everything else untouched. */
export function capitalise(value: string) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

/**
 * Underscores and dashes to spaces.
 *
 * Machine identifiers reach the UI in several dialects — `nsg_attached` from an
 * edge kind, `storage_public_blob_access` from a query name, `field-report`
 * from a theme stem — and each site was doing its own `replaceAll`.
 */
export function spaced(value: string) {
  return value.replaceAll(/[_-]+/g, " ");
}

/** `spaced`, then capitalised: `field-report` becomes `Field report`. */
export function sentenceCase(value: string) {
  return capitalise(spaced(value));
}
