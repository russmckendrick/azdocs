/**
 * Every word the app shows.
 *
 * The source of truth is `data/labels/en.toml` in the root crate, loaded in
 * Rust and handed over on bootstrap already merged with the user's overrides
 * from `<config dir>/azdocs/labels/`. `generated-labels.json` is the built-in
 * set, written by `cargo test -p azdocs-desktop`; it gives this module its
 * type (a misspelt path is a `tsc` error) and the words to show before
 * bootstrap answers and in the browser preview, which has no Rust.
 *
 * Labels are immutable for the session — an edited override file needs a
 * restart — so a module-level store is enough. `useLabels` exists so
 * components read them the way they would read a context, and a context can
 * replace this without touching call sites.
 */

import defaults from "./generated-labels.json";

export type Labels = typeof defaults;

export const DEFAULT_LABELS: Labels = defaults;

let current: Labels = DEFAULT_LABELS;

function withDefaults(defaults: unknown, incoming: unknown): unknown {
  if (
    defaults === null ||
    typeof defaults !== "object" ||
    Array.isArray(defaults)
  )
    return incoming ?? defaults;
  const values =
    incoming !== null && typeof incoming === "object"
      ? (incoming as Record<string, unknown>)
      : {};
  return Object.fromEntries(
    Object.entries(defaults).map(([key, value]) => [
      key,
      withDefaults(value, values[key]),
    ]),
  );
}

/** Install the bootstrap payload's labels. Call before the first render that reads them. */
export function installLabels(next: Labels) {
  // A running desktop process can predate a hot-reloaded frontend's new copy.
  current = withDefaults(DEFAULT_LABELS, next) as Labels;
}

/** The installed labels, for code that runs outside a component. */
export function labels(): Labels {
  return current;
}

/** The installed labels, from a component. */
export function useLabels(): Labels {
  return current;
}
