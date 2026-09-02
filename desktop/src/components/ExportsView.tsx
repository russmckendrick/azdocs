import { useState } from "react";
import {
  Check,
  FileOutput,
  FilePenLine,
  FileText,
  FolderOpen,
  LoaderCircle,
  Network,
  Table2,
} from "lucide-react";
import { chooseExportDirectory, exportSnapshot, isTauri } from "../api";
import type {
  EstateSnapshot,
  ExportEvent,
  ExportResult,
} from "../types";
import { errorMessage, fill, plural } from "../format";
import { useLabels } from "../labels";
import { ViewHeading } from "./view-chrome";
import { EXPORT_PRESETS, type ExportPresetId } from "./export-presets";

const DEFAULT_PRESET = EXPORT_PRESETS[0]!;

function PresetIcon({ id }: { id: ExportPresetId }) {
  if (id === "field-report") return <FileText size={20} />;
  if (id === "word-report") return <FilePenLine size={20} />;
  if (id === "data-workbook") return <Table2 size={20} />;
  return <Network size={20} />;
}

function relativeOutput(result: ExportResult, output: string) {
  const root = result.destination.replace(/[\\/]+$/, "");
  return output.startsWith(root) ? output.slice(root.length).replace(/^[\\/]/, "") : output;
}

export function ExportsView({ estate }: { estate: EstateSnapshot }) {
  const [presetId, setPresetId] = useState<ExportPresetId>("field-report");
  const [destination, setDestination] = useState("");
  const words = useLabels().desktop.exports;
  const [running, setRunning] = useState(false);
  const [message, setMessage] = useState(words.choose_directory);
  const [result, setResult] = useState<ExportResult>();
  const [error, setError] = useState<string>();

  const preset = EXPORT_PRESETS.find((item) => item.id === presetId) ?? DEFAULT_PRESET;
  const copy = words.presets[preset.id];

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
    const outputDirectory = destination || await chooseDestination();
    if (!outputDirectory) return;
    setRunning(true);
    setResult(undefined);
    setError(undefined);
    setMessage(words.preparing);
    try {
      const next = await exportSnapshot({
        snapshotId: estate.id,
        destination: outputDirectory,
        exportKind: preset.exportKind,
        formats: preset.formats,
        diagramType: preset.diagramType,
      }, handleEvent);
      setResult(next);
    } catch (caught) {
      const detail = errorMessage(caught, words.failed);
      setError(detail);
      setMessage(words.incomplete);
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="exports-workspace">
      <ViewHeading
        title={words.title}
        description={words.description}
        modifier="export-heading"
      >
        <div className="export-snapshot" aria-label={words.source_aria}>
          <span>{words.snapshot}</span>
          <strong>{fill(words.resources, { count: estate.resources.length.toLocaleString() })}</strong>
          <small className="mono">{estate.id}</small>
        </div>
      </ViewHeading>

      <div className="export-layout">
        <div className="export-composer">
          <section className="export-section" aria-labelledby="export-deliverable-heading">
            <div className="export-section-heading">
              <div>
                <h2 id="export-deliverable-heading">{words.choose_title}</h2>
                <p>{words.choose_detail}</p>
              </div>
            </div>

            <fieldset className="export-deliverable-list">
              <legend className="sr-only">{words.legend}</legend>
              {EXPORT_PRESETS.map((item) => {
                const selected = item.id === presetId;
                const itemCopy = words.presets[item.id];
                return (
                  <label key={item.id} className={selected ? "export-deliverable-row selected" : "export-deliverable-row"}>
                    <input
                      type="radio"
                      name="export-deliverable"
                      value={item.id}
                      checked={selected}
                      onChange={() => selectPreset(item.id)}
                    />
                    <span className="export-deliverable-icon" aria-hidden="true"><PresetIcon id={item.id} /></span>
                    <span className="export-deliverable-copy">
                      <span><strong>{itemCopy.label}</strong><code>{item.extension}</code></span>
                      <small>{itemCopy.detail}</small>
                      <small className="export-deliverable-use">{itemCopy.includes}</small>
                    </span>
                    <span className="export-choice" aria-hidden="true">{selected ? <Check size={14} /> : null}</span>
                  </label>
                );
              })}
            </fieldset>

            <p className="export-advanced-note">{words.advanced_note}</p>
          </section>
        </div>

        <aside className="export-dispatch" aria-live="polite">
          <div className="export-dispatch-heading">
            <FileOutput size={20} />
            <div><h2>{words.run_title}</h2><p>{copy.label}</p></div>
          </div>

          <div className="export-destination">
            <span>{words.destination}</span>
            <code title={destination}>{destination || words.not_selected}</code>
            <button className="quiet-button" onClick={() => void chooseDestination()} disabled={running}>
              <FolderOpen size={15} />
              {words.choose_folder}
            </button>
          </div>

          <dl className="export-summary">
            <div><dt>{words.source}</dt><dd>{fill(words.source_value, { status: estate.status })}</dd></div>
            <div><dt>{words.deliverable}</dt><dd>{copy.label}</dd></div>
            <div><dt>{words.format}</dt><dd className="mono">{preset.extension}</dd></div>
            <div><dt>{words.access}</dt><dd>{words.access_value}</dd></div>
          </dl>

          <button className="export-run-button" onClick={() => void runExport()} disabled={running}>
            {running ? <LoaderCircle className="spin" size={16} /> : <FileOutput size={16} />}
            {running
              ? words.generating
              : fill(isTauri ? words.run_action : words.preview_action, { preset: copy.action })}
          </button>
          <p className={error ? "export-run-status error" : "export-run-status"}>{error ?? message}</p>

          {result ? (
            <div className="export-receipt">
              <strong><Check size={14} /> {words.complete}</strong>
              <ul>
                {result.outputs.map((output) => (
                  <li key={output}><code title={output}>{relativeOutput(result, output)}</code></li>
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
