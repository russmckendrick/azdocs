import type { ReactNode } from "react";

/**
 * The heading every workspace opens with.
 *
 * Seven components repeated `<header className="view-heading"><div><h1>…</h1>
 * <p>…</p></div>…</header>`, some with a modifier class and some with an aside
 * (the SQLite stamp, the export snapshot picker). `description` is a ReactNode
 * because several of them interpolate a `<span className="mono">` into it.
 */
export function ViewHeading({
  title,
  description,
  modifier,
  children,
}: {
  title: string;
  description?: ReactNode;
  /** Extra class alongside `view-heading`, e.g. `findings-heading`. */
  modifier?: string;
  /** Right-hand aside — a database stamp, a snapshot picker. */
  children?: ReactNode;
}) {
  return (
    <header className={modifier ? `view-heading ${modifier}` : "view-heading"}>
      <div>
        <h1>{title}</h1>
        {description === undefined ? null : <p>{description}</p>}
      </div>
      {children}
    </header>
  );
}

/**
 * "Nothing here" for a pane, shelf or canvas.
 *
 * Seven of these existed with the same `icon + <strong> + <span>` shape. The
 * container class stays a prop rather than being unified: `no-results`,
 * `graph-empty-state` and `tree-empty-row` are deliberately styled differently
 * — a full pane, a canvas overlay and a one-line shelf row are not the same
 * thing, and collapsing their CSS would change how the app looks.
 *
 * `App`'s `empty-workspace` is not built from this: it carries actions and an
 * `h1/p` pair, so it is a different component rather than a variant.
 */
export function EmptyState({
  className,
  icon,
  title,
  detail,
  detailClassName,
  role,
}: {
  className: string;
  icon?: ReactNode;
  title?: string;
  detail?: ReactNode;
  /** e.g. `muted-copy` on the inventory variant. */
  detailClassName?: string;
  role?: "status";
}) {
  return (
    <div className={className} role={role}>
      {icon}
      {title ? <strong>{title}</strong> : null}
      {detail === undefined ? null : <span className={detailClassName}>{detail}</span>}
    </div>
  );
}

/**
 * The "this came from a file on disk, not Azure" stamp in a view heading.
 *
 * Three headings carried it with the same markup and a different icon.
 */
export function DatabaseStamp({
  icon,
  label,
  value,
}: {
  /** Absent on the overview, which leans on the heading beside it. */
  icon?: ReactNode;
  label: string;
  value: ReactNode;
}) {
  return (
    <div className="database-stamp">
      {icon}
      <span>
        <small>{label}</small>
        <strong>{value}</strong>
      </span>
    </div>
  );
}
