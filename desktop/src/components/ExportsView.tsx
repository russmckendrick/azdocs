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
  ExportKind,
  ExportResult,
} from "../types";
import { errorMessage } from "../format";
import { ViewHeading } from "./view-chrome";

type ExportPresetId = "field-report" | "word-report" | "data-workbook" | "diagram-workbook";

interface ExportPreset {
  id: ExportPresetId;
  label: string;
  actionLabel: string;
  extension: string;
  detail: string;
  includes: string;
  exportKind: ExportKind;
  formats: string[];
  diagramType?: string;
}

const EXPORT_PRESETS: ExportPreset[] = [
  {
    id: "field-report",
    label: "Field Report",
    actionLabel: "Field Report",
    extension: ".pdf",
    detail: "Print-ready estate review with findings, governance and diagrams.",
    includes: "Best for review packs, sign-off and long-term filing.",
    exportKind: "reports",
    formats: ["pdf"],
  },
  {
    id: "word-report",
    label: "Word Report",
    actionLabel: "Word Report",
    extension: ".docx",
    detail: "The same evidence and structure in an editable document.",
    includes: "Best when the report needs commentary or local editing.",
    exportKind: "reports",
    formats: ["docx"],
  },
  {
    id: "data-workbook",
    label: "Data Workbook",
    actionLabel: "Data Workbook",
    extension: ".xlsx",
    detail: "Inventory, findings, governance and query results as worksheets.",
    includes: "Best for filtering, reconciliation and further analysis.",
    exportKind: "reports",
    formats: ["xlsx"],
  },
  {
    id: "diagram-workbook",
    label: "Diagram Workbook",
    actionLabel: "Diagram Workbook",
    extension: ".drawio",
    detail: "Editable hierarchy, network, VNet and resource-group sheets.",
    includes: "Best for architecture review and diagram hand-off.",
    exportKind: "diagrams",
    formats: ["drawio"],
    diagramType: "workbook",
  },
];

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
  const [running, setRunning] = useState(false);
  const [message, setMessage] = useState("Choose an output directory.");
  const [result, setResult] = useState<ExportResult>();
  const [error, setError] = useState<string>();

  const preset = EXPORT_PRESETS.find((item) => item.id === presetId) ?? DEFAULT_PRESET;

  function invalidateRun() {
    setResult(undefined);
    setError(undefined);
    setMessage("Choose an output directory.");
  }

  function selectPreset(value: ExportPresetId) {
    setPresetId(value);
    invalidateRun();
  }

  function handleEvent(event: ExportEvent) {
    if (event.event === "phase") setMessage(event.data.message);
    if (event.event === "complete") {
      setMessage(`Exported ${event.data.outputCount} artifact${event.data.outputCount === 1 ? "" : "s"}.`);
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
    setMessage("Preparing the stored snapshot");
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
      const detail = errorMessage(caught, "The export could not be completed.");
      setError(detail);
      setMessage("Export did not complete.");
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="exports-workspace">
      <ViewHeading
        title="Exports"
        description="Produce a hand-off-ready artifact from the active stored snapshot. Generation stays offline."
        modifier="export-heading"
      >
        <div className="export-snapshot" aria-label="Export source">
          <span>Snapshot</span>
          <strong>{estate.resources.length.toLocaleString()} resources</strong>
          <small className="mono">{estate.id}</small>
        </div>
      </ViewHeading>

      <div className="export-layout">
        <div className="export-composer">
          <section className="export-section" aria-labelledby="export-deliverable-heading">
            <div className="export-section-heading">
              <div>
                <h2 id="export-deliverable-heading">Choose a deliverable</h2>
                <p>Each option has one clear purpose and a stable output format.</p>
              </div>
            </div>

            <fieldset className="export-deliverable-list">
              <legend className="sr-only">Export deliverable</legend>
              {EXPORT_PRESETS.map((item) => {
                const selected = item.id === presetId;
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
                      <span><strong>{item.label}</strong><code>{item.extension}</code></span>
                      <small>{item.detail}</small>
                      <small className="export-deliverable-use">{item.includes}</small>
                    </span>
                    <span className="export-choice" aria-hidden="true">{selected ? <Check size={14} /> : null}</span>
                  </label>
                );
              })}
            </fieldset>

            <p className="export-advanced-note">
              Need HTML, CSV, Markdown, scoped diagrams or image exports? Those specialist formats remain available in the CLI.
            </p>
          </section>
        </div>

        <aside className="export-dispatch" aria-live="polite">
          <div className="export-dispatch-heading">
            <FileOutput size={20} />
            <div><h2>Export run</h2><p>{preset.label}</p></div>
          </div>

          <div className="export-destination">
            <span>Output directory</span>
            <code title={destination}>{destination || "Not selected"}</code>
            <button className="quiet-button" onClick={() => void chooseDestination()} disabled={running}>
              <FolderOpen size={15} />
              Choose folder…
            </button>
          </div>

          <dl className="export-summary">
            <div><dt>Source</dt><dd>{estate.status} snapshot</dd></div>
            <div><dt>Deliverable</dt><dd>{preset.label}</dd></div>
            <div><dt>Format</dt><dd className="mono">{preset.extension}</dd></div>
            <div><dt>Access</dt><dd>SQLite only</dd></div>
          </dl>

          <button className="export-run-button" onClick={() => void runExport()} disabled={running}>
            {running ? <LoaderCircle className="spin" size={16} /> : <FileOutput size={16} />}
            {running ? "Generating export" : isTauri ? `Export ${preset.actionLabel}` : `Preview ${preset.actionLabel}`}
          </button>
          <p className={error ? "export-run-status error" : "export-run-status"}>{error ?? message}</p>

          {result ? (
            <div className="export-receipt">
              <strong><Check size={14} /> Export complete</strong>
              <ul>
                {result.outputs.map((output) => (
                  <li key={output}><code title={output}>{relativeOutput(result, output)}</code></li>
                ))}
              </ul>
              {!isTauri ? <small>Preview only — the browser workspace does not write files.</small> : null}
            </div>
          ) : null}
        </aside>
      </div>
    </div>
  );
}
