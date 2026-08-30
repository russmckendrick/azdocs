import type { AppBootstrap, EstateSnapshot, ThemePreference } from "../types";
import { dayMonthYear } from "../format";
import { ViewHeading } from "./view-chrome";

const THEME_OPTIONS: Array<{ id: ThemePreference; label: string }> = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];


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

  return (
    <div className="settings-workspace">
      <ViewHeading
        title="Settings"
        description="Preferences live on this machine; nothing here touches the stored snapshots or Azure."
      />

      <section className="settings-section">
        <h2>Appearance</h2>
        <div className="set-row">
          <div className="set-name">
            <b>Theme</b>
            <span>System follows the operating system; the report look is the light presentation.</span>
          </div>
          <div className="set-value" style={{ flexDirection: "column", alignItems: "flex-start" }}>
            <div className="seg" role="radiogroup" aria-label="Theme">
              {THEME_OPTIONS.map((option) => (
                <button
                  key={option.id}
                  role="radio"
                  aria-checked={themePreference === option.id}
                  className={themePreference === option.id ? "on" : ""}
                  onClick={() => onThemeChange(option.id)}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <span className="muted-copy">
              Currently resolving to <strong style={{ color: "var(--ink)" }}>{resolvedTheme}</strong>
              {themePreference === "system" ? " — following the operating system." : "."}
            </span>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h2>Database</h2>
        <div className="set-row">
          <div className="set-name">
            <b>Snapshot store</b>
            <span>The SQLite file every screen reads from.</span>
          </div>
          <div className="set-value">
            <span className="field" title={bootstrap.databasePath}>{bootstrap.databasePath}</span>
            <button className="quiet-button" onClick={onOpenDatabase}>Open another…</button>
          </div>
        </div>
        <div className="set-row">
          <div className="set-name">
            <b>Snapshots held</b>
            <span>Prune old snapshots with the CLI: <span className="mono">azdocs snapshots delete</span>.</span>
          </div>
          <div className="set-value">
            <span>
              <strong style={{ fontFamily: "var(--font-serif)", fontSize: 15, color: "var(--ink)" }}>
                {bootstrap.snapshots.length}
              </strong>{" "}
              snapshot{bootstrap.snapshots.length === 1 ? "" : "s"}
              {oldest && newest ? ` · ${dayMonthYear(oldest.createdAt)} – ${dayMonthYear(newest.createdAt)}` : ""}
            </span>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h2>Collection</h2>
        <div className="set-row">
          <div className="set-name">
            <b>Configuration</b>
            <span>Credentials stay in the file, never in the app.</span>
          </div>
          <div className="set-value">
            <span className="field" title={bootstrap.configPath}>{bootstrap.configPath}</span>
            {bootstrap.hasCredentials ? (
              <span className="set-good">● credentials found</span>
            ) : (
              <span className="set-warn">○ credentials missing</span>
            )}
          </div>
        </div>
        <div className="set-row">
          <div className="set-name">
            <b>Required tags</b>
            <span>Drives the governance sweep and the missing-required-tags audit.</span>
          </div>
          <div className="set-value">
            {bootstrap.requiredTags.length > 0 ? (
              bootstrap.requiredTags.map((tag) => (
                <span key={tag} className="tag-chip">{tag}</span>
              ))
            ) : (
              <span className="muted-copy">None configured — add <span className="mono">[audit] required_tags</span> to azdocs.toml.</span>
            )}
          </div>
        </div>
        {estate ? (
          <div className="set-row">
            <div className="set-name">
              <b>Active snapshot</b>
              <span>Everything on screen reads from this stored observation.</span>
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
