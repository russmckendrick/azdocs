/**
 * Per-viewer conveniences kept in localStorage: the theme, the sidebar
 * state, the estate explorer's filters. Nothing here is state the app
 * relies on — every read falls back, every write may silently fail (private
 * windows, cleared site data), and a stale value is harmless.
 *
 * Keys are scoped by tenant where the value describes one estate, so two
 * tenants do not share a filter that names a subscription of only one.
 */

const PREFIX = "azdocs-";

export function readPreference<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(PREFIX + key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

export function writePreference<T>(key: string, value: T) {
  try {
    window.localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch {
    // The preference still applies for this session.
  }
}

export function clearPreference(key: string) {
  try {
    window.localStorage.removeItem(PREFIX + key);
  } catch {
    // Nothing to clear.
  }
}

/** A key for state that belongs to one tenant's estate. */
export function tenantKey(tenantId: string | null | undefined, key: string) {
  return `${key}:${tenantId ?? "no-tenant"}`;
}
