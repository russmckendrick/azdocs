import { useEffect, useState } from "react";

export interface ProgressiveList<T> {
  visible: T[];
  total: number;
  /** How many the next press would add — never more than remain. */
  remaining: number;
  hasMore: boolean;
  showMore: () => void;
}

/**
 * Reveal a long list in batches.
 *
 * Four workspaces had their own copy of this — a `*_BATCH` const, a count in
 * state, a `useEffect` resetting it when the filters change, a `slice`, and a
 * "Show N more" button whose markup was byte-identical in three of them. The
 * batch sizes differed (200 vs 250) for no reason anyone recorded, so this keeps
 * one default and lets a caller override it.
 *
 * `resetKeys` is the dependency list meaning "the underlying set changed, start
 * from the top" — changing a filter should not leave the reader scrolled into a
 * list that no longer exists.
 */
export function useProgressiveList<T>(
  items: T[],
  resetKeys: unknown[],
  batch = 200,
): ProgressiveList<T> {
  const [visibleCount, setVisibleCount] = useState(batch);

  useEffect(() => {
    setVisibleCount(batch);
    // The caller decides what invalidates the window; `batch` is stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, resetKeys);

  const shown = Math.min(visibleCount, items.length);
  return {
    visible: items.slice(0, visibleCount),
    total: items.length,
    remaining: Math.min(batch, items.length - shown),
    hasMore: visibleCount < items.length,
    showMore: () => setVisibleCount((current) => current + batch),
  };
}

