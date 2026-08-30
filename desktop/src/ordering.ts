import type { Severity } from "./types";

/**
 * Severity order, high first — the same order `Severity` derives in
 * src/model/mod.rs and the store's `ORDER BY` uses.
 *
 * This list was retyped in four places and the rank derived from an inline
 * `indexOf` in one of them.
 */
export const SEVERITIES: Severity[] = ["high", "medium", "low", "info"];

/** Sort key for a severity; unknown values sort last rather than first. */
export function severityRank(severity: string) {
  const index = SEVERITIES.indexOf(severity as Severity);
  return index === -1 ? SEVERITIES.length : index;
}

/**
 * The design token a severity is drawn in.
 *
 * Colour lives in the token layer, not here — this maps a severity onto the
 * `var(--…)` name so a component that needs an inline style agrees with the
 * `.severity-*` CSS classes the other components use.
 */
export const SEVERITY_TOKEN: Record<Severity, string> = {
  high: "var(--coral)",
  medium: "var(--amber)",
  low: "var(--muted)",
  info: "var(--accent)",
};

/**
 * Total, deterministic string order.
 *
 * `localeCompare` is locale-dependent and can return 0 for strings that differ,
 * which would make a sort unstable across machines. The topology layout,
 * presentation and model layers each had a private copy of this because their
 * output has to be identical run to run.
 */
export function stableCompare(left: string, right: string) {
  return left < right ? -1 : left > right ? 1 : 0;
}
