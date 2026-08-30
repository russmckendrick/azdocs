/**
 * The frontend's view of the app's types.
 *
 * Three layers, and it matters which one a type belongs in:
 *
 * - `./generated` — the wire contract, emitted from the Rust DTOs by
 *   `cargo test -p azdocs-desktop`. Never edit it; change the struct instead.
 * - `./api-types` — closed string unions Rust serialises through `as_str()`,
 *   which ts-rs cannot infer. Hand-written, referenced from the Rust structs.
 * - This file — types that exist only in the UI and have no Rust counterpart.
 *
 * Import from `./types` everywhere; the split is an implementation detail.
 */

export * from "./api-types";
export * from "./generated";

/** Which workspace the shell is showing. Purely a frontend concern. */
export type ViewId =
  | "overview"
  | "estate"
  | "topology"
  | "inventory"
  | "findings"
  | "governance"
  | "history"
  | "exports"
  | "settings";

/** Tri-state theme, persisted in Settings. No Rust counterpart. */
export type ThemePreference = "system" | "light" | "dark";

/** A user's current subscription / resource-group narrowing. */
export interface ScopeSelection {
  subscriptionId?: string;
  resourceGroup?: string;
}
