import { useEffect, useState } from "react";
import {
  Check,
  Database,
  FileOutput,
  FilePenLine,
  FileText,
  FolderOpen,
  LoaderCircle,
  Network,
  Square,
  Table2,
} from "lucide-react";
import {
  cancelExport,
  chooseExportDirectory,
  exportSnapshot,
  getReportThemes,
  isTauri,
  openExportFolder,
  revealExportPath,
} from "../api";
import type {
  EstateSnapshot,
  ExportEvent,
  ExportResult,
  ReportThemes,
  Severity,
} from "../types";

const SEVERITIES: Severity[] = ["high", "medium", "low", "info"];
import { errorMessage, fill, plural, sentenceCase } from "../format";
import { useLabels } from "../labels";
import { DatabaseStamp, ViewHeading } from "./view-chrome";
import { EXPORT_PRESETS, type ExportPresetId } from "./export-presets";
import { ThemeSpecimen } from "./ThemeSpecimen";

const DEFAULT_PRESET = EXPORT_PRESETS[0]!;

function PresetIcon({ id }: { id: ExportPresetId }) {
  if (id === "field-report") return <FileText size={18} />;
  if (id === "word-report") return <FilePenLine size={18} />;
  if (id === "data-workbook") return <Table2 size={18} />;
  return <Network size={18} />;
}

function relativeOutput(result: ExportResult, output: string) {
  const root = result.destination.replace(/[\\/]+$/, "");
  return output.startsWith(root)
    ? output.slice(root.length).replace(/^[\\/]/, "")
    : output;
}

export function ExportsView({ estate }: { estate: EstateSnapshot }) {
  const [presetId, setPresetId] = useState<ExportPresetId>("field-report");
  const [includeReference, setIncludeReference] = useState(false);
  const [subscriptionId, setSubscriptionId] = useState("");
  const [resourceGroup, setResourceGroup] = useState("");
  const [minSeverity, setMinSeverity] = useState("");
  const [destination, setDestination] = useState("");
  const words = useLabels().desktop.exports;
  const [running, setRunning] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [message, setMessage] = useState(words.choose_directory);
  const [result, setResult] = useState<ExportResult>();
  const [error, setError] = useState<string>();
  const [themes, setThemes] = useState<ReportThemes>();
  const [themeName, setThemeName] = useState<string>();

  useEffect(() => {
    let current = true;
    getReportThemes(estate.id)
      .then((next) => {
        if (!current) return;
        setThemes(next);
        // Open on the configured theme; keep a choice already made.
        setThemeName(
          (chosen) =>
            chosen ??
            next.themes.find((theme) => theme.name === next.configured)?.name ??
            next.themes[0]?.name,
        );
      })
      // Without previews the export still uses the configured theme.
      .catch(() => {
        if (current) setThemes(undefined);
      });
    return () => {
      current = false;
    };
  }, [estate.id]);

  const preset =
    EXPORT_PRESETS.find((item) => item.id === presetId) ?? DEFAULT_PRESET;
  const copy = words.presets[preset.id];
  const printed = preset.formats.some(
    (format) => format === "pdf" || format === "docx",
  );
  const theme = themes?.themes.find((item) => item.name === themeName);
  const themeTitle = (item: { name: string; title: string }) =>
    item.title || sentenceCase(item.name);

  function invalidateRun() {
    setResult(undefined);
    setError(undefined);
    setMessage(words.choose_directory);
  }

  function selectPreset(value: ExportPresetId) {
    setPresetId(value);
    invalidateRun();
  }

  function handleEvent(event: ExportEvent) {
    if (event.event === "phase") setMessage(event.data.message);
    if (event.event === "complete") {
      setMessage(plural(words.exported, event.data.outputCount));
    }
    if (event.event === "failed") setMessage(event.data.message);
  }

  async function chooseDestination() {
    const path = await chooseExportDirectory();
    if (path) {
      setDestination(path);
      setResult(undefined);
      setError(undefined);
    }
    return path;
  }

  async function runExport() {
    if (running) return;
    const outputDirectory = destination || (await chooseDestination());
    if (!outputDirectory) return;
    setRunning(true);
    setResult(undefined);
    setError(undefined);
    setMessage(words.preparing);
    try {
      setStopping(false);
      const next = await exportSnapshot(
        {
          snapshotId: estate.id,
          destination: outputDirectory,
          exportKind: preset.exportKind,
          formats: preset.formats,
          diagramType: preset.diagramType,
          includeReference: printed && includeReference,
          subscriptionId: subscriptionId || null,
          resourceGroup: resourceGroup || null,
          minSeverity:
            preset.exportKind === "reports" && minSeverity
              ? (minSeverity as Severity)
              : null,
          theme: printed && theme ? theme.name : null,
        },
        handleEvent,
      );
      setResult(next);
      if (next.cancelled) setMessage(words.cancelled);
    } catch (caught) {
      const detail = errorMessage(caught, words.failed);
      setError(detail);
      setMessage(words.incomplete);
    } finally {
      setRunning(false);
      setStopping(false);
    }
  }

  async function stopExport() {
    setStopping(true);
    try {
      await cancelExport();
    } catch (caught) {
      setError(errorMessage(caught, words.failed));
    }
  }

  return (
    <div className="exports-workspace">
      <ViewHeading title={words.title} description={words.description}>
        <div role="group" aria-label={words.source_aria}>
          <DatabaseStamp
            icon={<Database size={16} />}
            label={words.snapshot}
            value={
              <span title={estate.id}>
                {fill(words.resources, {
                  count: estate.resources.length.toLocaleString(),
                })}
                {" · "}
                {estate.id.slice(0, 8)}
              </span>
            }
          />
        </div>
      </ViewHeading>

      <div className="export-layout">
        <div className="export-composer">
          <section
            className="export-section"
            aria-labelledby="export-deliverable-heading"
          >
            <header className="export-section-heading">
              <h2 id="export-deliverable-heading">{words.choose_title}</h2>
              <p>{words.choose_detail}</p>
            </header>

            <fieldset className="export-deliverable-list">
              <legend className="sr-only">{words.legend}</legend>
              {EXPORT_PRESETS.map((item) => {
                const selected = item.id === presetId;
                const itemCopy = words.presets[item.id];
                return (
                  <label
                    key={item.id}
                    className={
                      selected
                        ? "export-deliverable-row selected"
                        : "export-deliverable-row"
                    }
                  >
                    <input
                      type="radio"
                      name="export-deliverable"
                      value={item.id}
                      checked={selected}
                      onChange={() => selectPreset(item.id)}
                    />
                    <span
                      className="export-deliverable-icon"
                      aria-hidden="true"
                    >
                      <PresetIcon id={item.id} />
                    </span>
                    <span className="export-deliverable-copy">
                      <span>
                        <strong>{itemCopy.label}</strong>
                        <code>{item.extension}</code>
                      </span>
                      <small>{itemCopy.detail}</small>
                      <small className="export-deliverable-use">
                        {itemCopy.includes}
                      </small>
                    </span>
                    <span className="export-choice" aria-hidden="true">
                      {selected ? <Check size={14} /> : null}
                    </span>
                  </label>
                );
              })}
            </fieldset>

            {printed && (
              <label className="export-reference-option">
                <input
                  type="checkbox"
                  checked={includeReference}
                  disabled={running}
                  onChange={(event) => {
                    setIncludeReference(event.target.checked);
                    invalidateRun();
                  }}
                />
                <span>
                  <strong>{words.include_reference}</strong>
                  <small>{words.reference_detail}</small>
                </span>
              </label>
            )}
            {printed && themes && themes.themes.length > 0 ? (
              <fieldset className="export-style">
                <legend className="export-section-heading">
                  <h2>{words.theme_title}</h2>
                </legend>
                <p>{words.theme_detail}</p>
                <div
                  className="export-style-options"
                  role="radiogroup"
                  aria-label={words.theme_legend}
                >
                  {themes.themes.map((item) => {
                    const selected = item.name === themeName;
                    return (
                      <label
                        key={item.name}
                        className={
                          selected
                            ? "export-style-option selected"
                            : "export-style-option"
                        }
                      >
                        <input
                          type="radio"
                          name="export-style"
                          value={item.name}
                          checked={selected}
                          disabled={running}
                          onChange={() => {
                            setThemeName(item.name);
                            invalidateRun();
                          }}
                        />
                        <ThemeSpecimen theme={item} />
                        <span className="export-style-copy">
                          <span>
                            <strong>{themeTitle(item)}</strong>
                            {item.name === themes.configured ? (
                              <small className="export-style-configured">
                                {words.theme_configured}
                              </small>
                            ) : null}
                          </span>
                          <small>{item.description}</small>
                        </span>
                      </label>
                    );
                  })}
                </div>
              </fieldset>
            ) : null}
            <fieldset className="export-scope">
              <legend className="export-section-heading">
                <h2>{words.scope_title}</h2>
              </legend>
              <p>{words.scope_note}</p>
              <div className="export-scope-fields">
              <label>
                <span>{words.scope_subscription}</span>
                <select
                  value={subscriptionId}
                  disabled={running}
                  onChange={(event) => {
                    setSubscriptionId(event.target.value);
                    setResourceGroup("");
                    invalidateRun();
                  }}
                >
                  <option value="">{words.scope_any}</option>
                  {estate.subscriptions.map((subscription) => (
                    <option
                      key={subscription.id}
                      value={subscription.id}
                    >
                      {subscription.displayName}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>{words.scope_resource_group}</span>
                <select
                  value={resourceGroup}
                  disabled={running}
                  onChange={(event) => {
                    setResourceGroup(event.target.value);
                    invalidateRun();
                  }}
                >
                  <option value="">{words.scope_any}</option>
                  {estate.resourceGroups
                    .filter(
                      (group) =>
                        !subscriptionId ||
                        group.subscriptionId === subscriptionId,
                    )
                    .map((group) => (
                      <option
                        key={`${group.subscriptionId}/${group.name}`}
                        value={group.name}
                      >
                        {group.name}
                      </option>
                    ))}
                </select>
              </label>
              {preset.exportKind === "reports" ? (
                <label>
                  <span>{words.scope_severity}</span>
                  <select
                    value={minSeverity}
                    disabled={running}
                    onChange={(event) => {
                      setMinSeverity(event.target.value);
                      invalidateRun();
                    }}
                  >
                    <option value="">{words.scope_any}</option>
                    {SEVERITIES.map((severity) => (
                      <option key={severity} value={severity}>
                        {fill(words.scope_severity_option, { severity })}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              </div>
            </fieldset>
            <p className="export-advanced-note">{words.advanced_note}</p>
          </section>
        </div>

        <aside className="export-dispatch" aria-live="polite">
          <div className="export-dispatch-heading">
            <FileOutput size={20} />
            <div>
              <h2>{words.run_title}</h2>
              <p>{copy.label}</p>
            </div>
          </div>

          <div className="export-destination">
            <span>{words.destination}</span>
            <code title={destination}>{destination || words.not_selected}</code>
            <button
              className="quiet-button"
              onClick={() => void chooseDestination()}
              disabled={running}
            >
              <FolderOpen size={15} />
              {words.choose_folder}
            </button>
          </div>

          <dl className="export-summary">
            <div>
              <dt>{words.source}</dt>
              <dd>{fill(words.source_value, { status: estate.status })}</dd>
            </div>
            <div>
              <dt>{words.deliverable}</dt>
              <dd>{copy.label}</dd>
            </div>
            {printed && theme ? (
              <div>
                <dt>{words.theme}</dt>
                <dd>{themeTitle(theme)}</dd>
              </div>
            ) : null}
            <div>
              <dt>{words.format}</dt>
              <dd className="mono">{preset.extension}</dd>
            </div>
            <div>
              <dt>{words.access}</dt>
              <dd>{words.access_value}</dd>
            </div>
          </dl>

          <button
            className="collect-button export-run-button"
            onClick={() => void runExport()}
            disabled={running}
          >
            {running ? (
              <LoaderCircle className="spin" size={16} />
            ) : (
              <FileOutput size={16} />
            )}
            {running
              ? words.generating
              : fill(isTauri ? words.run_action : words.preview_action, {
                  preset: copy.action,
                })}
          </button>
          {running ? (
            <button
              className="quiet-button"
              onClick={() => void stopExport()}
              disabled={stopping}
            >
              <Square size={14} />
              {words.cancel}
            </button>
          ) : null}
          <p
            className={error ? "export-run-status error" : "export-run-status"}
          >
            {error ?? message}
          </p>

          {result ? (
            <div className="export-receipt">
              <strong>
                <Check size={14} /> {words.complete}
              </strong>
              {isTauri ? (
                <button
                  className="quiet-button"
                  onClick={() => void openExportFolder(result.destination)}
                >
                  <FolderOpen size={14} />
                  {words.open_folder}
                </button>
              ) : null}
              <ul>
                {result.outputs.map((output) => (
                  <li key={output}>
                    <code title={output}>{relativeOutput(result, output)}</code>
                    {isTauri ? (
                      <button
                        className="text-link"
                        onClick={() => void revealExportPath(output)}
                      >
                        {words.reveal}
                      </button>
                    ) : null}
                  </li>
                ))}
              </ul>
              {!isTauri ? <small>{words.preview_note}</small> : null}
            </div>
          ) : null}
        </aside>
      </div>
    </div>
  );
}
