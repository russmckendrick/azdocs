import type { CSSProperties } from "react";
import type { ReportTheme } from "../types";

/**
 * A two-page miniature of a document theme: its cover and a body page, drawn
 * from the theme's resolved palette and layout strategies. Every colour here
 * is data from `report_themes`, handed to styles.css as `--rt-*` properties;
 * the stylesheet decides how each strategy is drawn.
 */
export function ThemeSpecimen({ theme }: { theme: ReportTheme }) {
  const p = theme.palette;
  const style = {
    "--rt-primary": p.primary,
    "--rt-primary-dark": p.primaryDark,
    "--rt-tint": p.primaryTint,
    "--rt-accent": p.accent,
    "--rt-on-primary": p.onPrimary,
    "--rt-band": p.band,
    "--rt-on-band": p.onBand,
    "--rt-ink": p.ink,
    "--rt-muted": p.muted,
    "--rt-rule": p.rule,
    "--rt-surface": p.surface,
    "--rt-zebra": p.zebra,
    "--rt-high": p.high,
    "--rt-high-fill": p.highFill,
    "--rt-medium": p.medium,
    "--rt-medium-fill": p.mediumFill,
  } as CSSProperties;
  // An <img> never runs script, so theme artwork cannot reach the app.
  const art = theme.coverArt
    ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(theme.coverArt)}`
    : undefined;

  return (
    <span
      className="theme-specimen"
      style={style}
      data-cover={theme.cover}
      data-table={theme.table}
      data-stat={theme.stat}
      data-zebra={theme.zebraRows ? "on" : "off"}
      aria-hidden="true"
    >
      <span className="theme-page theme-cover">
        <span className="theme-cover-fill">
          {art ? <img src={art} alt="" draggable={false} /> : null}
        </span>
        <span className="theme-cover-text">
          <i className="theme-bar title" />
          <i className="theme-bar rule" />
          <i className="theme-bar subtitle" />
        </span>
        <i className="theme-bar meta" />
      </span>
      <span className="theme-page theme-body">
        <i className="theme-bar header" />
        <i className="theme-bar heading" />
        <span className="theme-stats">
          <i />
          <i />
          <i />
        </span>
        <span className="theme-table">
          <i className="head" />
          <i />
          <i />
          <i />
          <i />
        </span>
        <span className="theme-signals">
          <i className="high" />
          <i className="medium" />
        </span>
      </span>
    </span>
  );
}
