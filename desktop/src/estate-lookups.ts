import { useEffect, useMemo } from "react";
import type { EstateSnapshot, ResourceType } from "./types";

/**
 * Azure type -> its display name, icon and colour.
 *
 * Four components built this same `useMemo` independently. It is derived data
 * over a snapshot, not component state, so it belongs beside the snapshot.
 */
export function useResourceTypeMap(
  estate: EstateSnapshot | undefined,
): Map<string, ResourceType> {
  return useMemo(
    () => new Map(estate?.resourceTypes.map((type) => [type.azureType, type]) ?? []),
    [estate?.resourceTypes],
  );
}

/** Subscription id -> display name. Three components derived this separately. */
export function useSubscriptionNames(
  estate: EstateSnapshot | undefined,
): Map<string, string> {
  return useMemo(
    () =>
      new Map(
        estate?.subscriptions.map((subscription) => [subscription.id, subscription.displayName]) ?? [],
      ),
    [estate?.subscriptions],
  );
}

/**
 * Dismiss an overlay on Escape while `active`.
 *
 * The findings and resource drawers had the same effect written out, and App
 * handles Escape inline in its own keydown switch. Listening on `window` rather
 * than the element means it works regardless of where focus currently sits,
 * which is the behaviour a drawer wants.
 */
export function useEscapeKey(active: boolean, onEscape: () => void) {
  useEffect(() => {
    if (!active) return;
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") onEscape();
    }
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
    // `onEscape` is a fresh closure each render; `active` is the real trigger.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);
}
