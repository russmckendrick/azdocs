import { fill } from "../format";
import { useLabels } from "../labels";
import type { ProgressiveList } from "./use-progressive-list";

/**
 * The "Show N more" control.
 *
 * `inline` is the compact variant the inventory grid uses inside its status
 * strip, which omits the loaded-count line.
 */
export function ShowMore<T>({ list, inline = false }: { list: ProgressiveList<T>; inline?: boolean }) {
  const words = useLabels().desktop.progressive;
  if (!list.hasMore) return null;
  return (
    <button className={inline ? "load-more inline" : "load-more"} onClick={list.showMore}>
      {fill(words.show_more, { count: list.remaining })}
      {inline ? null : (
        <span>
          {fill(words.loaded, { visible: list.visible.length.toLocaleString(), total: list.total.toLocaleString() })}
        </span>
      )}
    </button>
  );
}
