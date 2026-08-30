/**
 * Shared display formatting.
 *
 * Six `Intl.DateTimeFormat` wrappers lived across five components, two of them
 * the same option bag under different names, and `id.split("/").at(-1)` appeared
 * in six places. Formatters are also memoised here: constructing an
 * `Intl.DateTimeFormat` is not cheap and these run per row.
 */

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
