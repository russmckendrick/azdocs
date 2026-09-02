import type { AppBootstrap, EstateSnapshot, ThemePreference } from "../types";
import { dayMonthYear, fill, fillNodes, plural } from "../format";
import { useLabels } from "../labels";
import { ViewHeading } from "./view-chrome";

/** Labels come from `desktop.settings.theme_options` under the same ids. */
const THEME_OPTIONS: ThemePreference[] = ["system", "light", "dark"];


export function SettingsView({
  bootstrap,
  estate,
  themePreference,
  resolvedTheme,
  onThemeChange,
  onOpenDatabase,
}: {
  bootstrap: AppBootstrap;
  estate?: EstateSnapshot;
  themePreference: ThemePreference;
  resolvedTheme: "light" | "dark";
  onThemeChange: (preference: ThemePreference) => void;
  onOpenDatabase: () => void;
}) {
  const newest = bootstrap.snapshots[0];
  const oldest = bootstrap.snapshots.at(-1);
  const words = useLabels().desktop.settings;

  return (
    <div className="settings-workspace">
      <ViewHeading
        title={words.title}
        description={words.description}
      />

      <section className="settings-section">
        <h2>{words.appearance}</h2>
        <div className="set-row">
          <div className="set-name">
            <b>{words.theme}</b>
            <span>{words.theme_detail}</span>
          </div>
          <div className="set-value" style={{ flexDirection: "column", alignItems: "flex-start" }}>
            <div className="seg" role="radiogroup" aria-label={words.theme_aria}>
              {THEME_OPTIONS.map((option) => (
                <button
                  key={option}
                  role="radio"
                  aria-checked={themePreference === option}
                  className={themePreference === option ? "on" : ""}
                  onClick={() => onThemeChange(option)}
                >
                  {words.theme_options[option] ?? option}
                </button>
              ))}
            </div>
            <span className="muted-copy">
              {fillNodes(words.resolving, {
                theme: <strong style={{ color: "var(--ink)" }}>{resolvedTheme}</strong>,
                suffix: themePreference === "system" ? words.following_system : words.resolved_end,
              })}
            </span>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h2>{words.database}</h2>
        <div className="set-row">
          <div className="set-name">
            <b>{words.store}</b>
            <span>{words.store_detail}</span>
          </div>
          <div className="set-value">
            <span className="field" title={bootstrap.databasePath}>{bootstrap.databasePath}</span>
            <button className="quiet-button" onClick={onOpenDatabase}>{words.open_another}</button>
          </div>
        </div>
        <div className="set-row">
          <div className="set-name">
            <b>{words.snapshots_held}</b>
            <span>{fillNodes(words.prune_hint, { command: <span className="mono">{words.prune_command}</span> })}</span>
          </div>
          <div className="set-value">
            <span>
              <strong style={{ fontFamily: "var(--font-serif)", fontSize: 15, color: "var(--ink)" }}>
                {bootstrap.snapshots.length}
              </strong>{" "}
              {plural(words.snapshot_count, bootstrap.snapshots.length)}
              {oldest && newest ? fill(words.snapshot_range, { from: dayMonthYear(oldest.createdAt), to: dayMonthYear(newest.createdAt) }) : ""}
            </span>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h2>{words.collection}</h2>
        <div className="set-row">
          <div className="set-name">
            <b>{words.configuration}</b>
            <span>{words.configuration_detail}</span>
          </div>
          <div className="set-value">
            <span className="field" title={bootstrap.configPath}>{bootstrap.configPath}</span>
            {bootstrap.hasCredentials ? (
              <span className="set-good">{words.credentials_found}</span>
            ) : (
              <span className="set-warn">{words.credentials_missing}</span>
            )}
          </div>
        </div>
        <div className="set-row">
          <div className="set-name">
            <b>{words.required_tags}</b>
            <span>{words.required_tags_detail}</span>
          </div>
          <div className="set-value">
            {bootstrap.requiredTags.length > 0 ? (
              bootstrap.requiredTags.map((tag) => (
                <span key={tag} className="tag-chip">{tag}</span>
              ))
            ) : (
              <span className="muted-copy">{fillNodes(words.none_configured, { key: <span className="mono">{words.none_configured_key}</span> })}</span>
            )}
          </div>
        </div>
        {estate ? (
          <div className="set-row">
            <div className="set-name">
              <b>{words.active_snapshot}</b>
              <span>{words.active_snapshot_detail}</span>
            </div>
            <div className="set-value">
              <span className="mono" style={{ fontSize: 11.5 }}>{estate.id}</span>
              <span className="muted-copy">{estate.status} · {estate.tenantId}</span>
            </div>
          </div>
        ) : null}
      </section>
    </div>
  );
}
