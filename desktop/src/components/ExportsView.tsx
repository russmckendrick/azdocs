import { useMemo, useState } from "react";
import {
  Check,
  FileArchive,
  FileOutput,
  FolderOpen,
  LoaderCircle,
  Network,
} from "lucide-react";
import { chooseExportDirectory, exportSnapshot, isTauri } from "../api";
import type {
  AppBootstrap,
  DiagramExportFormat,
  DiagramExportType,
  EstateSnapshot,
  ExportEvent,
  ExportKind,
  ExportResult,
  ReportExportFormat,
} from "../types";
import { ViewHeading } from "./view-chrome";

const REPORT_FORMATS: Array<{
  id: ReportExportFormat;
  label: string;
  extension: string;
  detail: string;
}> = [
  { id: "pdf", label: "PDF report", extension: ".pdf", detail: "Print-ready, themed and diagrammed" },
  { id: "docx", label: "Word report", extension: ".docx", detail: "Editable report with the same evidence" },
  { id: "html", label: "HTML report", extension: ".html", detail: "Single report plus a browsable docs site" },
  { id: "xlsx", label: "Excel workbook", extension: ".xlsx", detail: "Inventory, findings and query sheets" },
  { id: "csv", label: "CSV tables", extension: ".csv", detail: "Separate inventory and findings files" },
  { id: "md", label: "Markdown docs", extension: ".md", detail: "Portable, version-control friendly pages" },
];

const DIAGRAM_TYPES: Array<{ id: DiagramExportType; label: string; detail: string }> = [
  { id: "workbook", label: "Draw.io workbook", detail: "Network, peerings, VNets and resource groups as sheets" },
  { id: "network", label: "Network topology", detail: "VNets, subnets, peerings and attached resources" },
  { id: "resources", label: "Resource relationships", detail: "Stored resource dependencies across the selected scope" },
  { id: "hierarchy", label: "Estate hierarchy", detail: "Subscriptions and resource groups" },
  { id: "vnets", label: "Per virtual network", detail: "One full-detail diagram for every VNet" },
  { id: "resource-groups", label: "Per resource group", detail: "One full-detail diagram for every resource group" },
];

const DIAGRAM_FORMATS: Array<{ id: DiagramExportFormat; label: string; detail: string }> = [
  { id: "drawio", label: "Draw.io", detail: "Editable source" },
  { id: "svg", label: "SVG", detail: "Scalable image" },
  { id: "png", label: "PNG", detail: "Raster image" },
  { id: "mermaid", label: "Mermaid", detail: "Text diagram" },
];

function toggle<T extends string>(values: T[], value: T) {
  return values.includes(value) ? values.filter((item) => item !== value) : [...values, value];
}

function sentenceCase(value: string) {
  return value.charAt(0).toUpperCase() + value.slice(1).replaceAll("-", " ");
}

function relativeOutput(result: ExportResult, output: string) {
  const root = result.destination.replace(/[\\/]+$/, "");
  return output.startsWith(root) ? output.slice(root.length).replace(/^[\\/]/, "") : output;
}

export function ExportsView({
  bootstrap,
  estate,
}: {
  bootstrap: AppBootstrap;
  estate: EstateSnapshot;
}) {
  const [kind, setKind] = useState<ExportKind>("reports");
  const [destination, setDestination] = useState("");
  const [reportFormats, setReportFormats] = useState<ReportExportFormat[]>(["pdf"]);
  const [theme, setTheme] = useState(bootstrap.reportTheme);
  const [diagramType, setDiagramType] = useState<DiagramExportType>("workbook");
  const [diagramFormats, setDiagramFormats] = useState<DiagramExportFormat[]>(["drawio"]);
  const [subscriptionId, setSubscriptionId] = useState("");
  const [resourceGroupId, setResourceGroupId] = useState("");
  const [running, setRunning] = useState(false);
  const [message, setMessage] = useState("Choose formats and an output directory.");
  const [result, setResult] = useState<ExportResult>();
  const [error, setError] = useState<string>();

  const resourceGroups = useMemo(
    () => estate.resourceGroups.filter((group) => !subscriptionId || group.subscriptionId === subscriptionId),
    [estate.resourceGroups, subscriptionId],
  );
  const selectedGroup = estate.resourceGroups.find((group) => group.id === resourceGroupId);
  const scopeSupported = diagramType !== "hierarchy";
  const selectedFormats = kind === "reports" ? reportFormats : diagramFormats;
  const canExport = selectedFormats.length > 0 && !running;
  const allReportFormatsSelected = reportFormats.length === REPORT_FORMATS.length;

  function invalidateRun() {
    setResult(undefined);
    setError(undefined);
    setMessage("Choose formats and an output directory.");
  }

  function selectKind(value: ExportKind) {
    setKind(value);
    invalidateRun();
  }

  function selectDiagramType(value: DiagramExportType) {
    setDiagramType(value);
    if (value === "workbook") {
      setDiagramFormats((current) => current.filter((format) => format !== "mermaid"));
    }
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
    if (!canExport) return;
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
        exportKind: kind,
        formats: selectedFormats,
        theme: kind === "reports" ? theme : undefined,
        diagramType: kind === "diagrams" ? diagramType : undefined,
        subscriptionId: kind === "diagrams" && scopeSupported
          ? (selectedGroup?.subscriptionId ?? subscriptionId) || undefined
          : undefined,
        resourceGroup: kind === "diagrams" && scopeSupported ? selectedGroup?.name : undefined,
      }, handleEvent);
      setResult(next);
    } catch (caught) {
      const detail = caught instanceof Error ? caught.message : String(caught);
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
        description="Turn the active stored snapshot into reports and diagrams. Generation stays offline."
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
          <div className="export-kind-switch" aria-label="Export family">
            <button
              aria-pressed={kind === "reports"}
              className={kind === "reports" ? "active" : ""}
              onClick={() => selectKind("reports")}
            >
              <FileArchive size={17} />
              <span><strong>Reports</strong><small>Documents and tables</small></span>
            </button>
            <button
              aria-pressed={kind === "diagrams"}
              className={kind === "diagrams" ? "active" : ""}
              onClick={() => selectKind("diagrams")}
            >
              <Network size={17} />
              <span><strong>Diagrams</strong><small>Editable and image formats</small></span>
            </button>
          </div>

          {kind === "reports" ? (
            <section className="export-section" aria-labelledby="report-formats-heading">
              <div className="export-section-heading">
                <div>
                  <h2 id="report-formats-heading">Report formats</h2>
                  <p>Select one or several. Shared diagrams are composed once.</p>
                </div>
                <button
                  className="text-action"
                  onClick={() => {
                    setReportFormats(allReportFormatsSelected ? [] : REPORT_FORMATS.map((format) => format.id));
                    invalidateRun();
                  }}
                >
                  {allReportFormatsSelected ? "Clear all" : "Select all"}
                </button>
              </div>
              <div className="export-format-list">
                {REPORT_FORMATS.map((format) => {
                  const selected = reportFormats.includes(format.id);
                  return (
                    <label key={format.id} className={selected ? "export-format-row selected" : "export-format-row"}>
                      <input
                        type="checkbox"
                        checked={selected}
                        onChange={() => {
                          setReportFormats(toggle(reportFormats, format.id));
                          invalidateRun();
                        }}
                      />
                      <span className="export-check" aria-hidden="true">{selected ? <Check size={13} /> : null}</span>
                      <span className="export-format-copy"><strong>{format.label}</strong><small>{format.detail}</small></span>
                      <code>{format.extension}</code>
                    </label>
                  );
                })}
              </div>
              <label className="export-select-row">
                <span><strong>Document theme</strong><small>Applied to HTML, Excel, PDF and Word.</small></span>
                <select
                  value={theme}
                  onChange={(event) => {
                    setTheme(event.target.value);
                    invalidateRun();
                  }}
                >
                  {bootstrap.reportThemes.map((name) => <option key={name} value={name}>{sentenceCase(name)}</option>)}
                </select>
              </label>
            </section>
          ) : (
            <section className="export-section" aria-labelledby="diagram-options-heading">
              <div className="export-section-heading">
                <div>
                  <h2 id="diagram-options-heading">Diagram set</h2>
                  <p>Standalone exports keep every resource at full detail.</p>
                </div>
              </div>
              <label className="export-select-row export-type-row">
                <span><strong>Composition</strong><small>{DIAGRAM_TYPES.find((item) => item.id === diagramType)?.detail}</small></span>
                <select value={diagramType} onChange={(event) => selectDiagramType(event.target.value as DiagramExportType)}>
                  {DIAGRAM_TYPES.map((type) => <option key={type.id} value={type.id}>{type.label}</option>)}
                </select>
              </label>
              <div className="export-format-list diagram-formats">
                {DIAGRAM_FORMATS.map((format) => {
                  const unavailable = diagramType === "workbook" && format.id === "mermaid";
                  const selected = diagramFormats.includes(format.id);
                  return (
                    <label key={format.id} className={selected ? "export-format-row selected" : "export-format-row"} aria-disabled={unavailable}>
                      <input
                        type="checkbox"
                        checked={selected}
                        disabled={unavailable}
                        onChange={() => {
                          setDiagramFormats(toggle(diagramFormats, format.id));
                          invalidateRun();
                        }}
                      />
                      <span className="export-check" aria-hidden="true">{selected ? <Check size={13} /> : null}</span>
                      <span className="export-format-copy"><strong>{format.label}</strong><small>{unavailable ? "No multi-sheet equivalent" : format.detail}</small></span>
                    </label>
                  );
                })}
              </div>

              <div className={scopeSupported ? "export-scope" : "export-scope disabled"}>
                <div>
                  <strong>Scope</strong>
                  <small>{scopeSupported ? "Optionally narrow the stored estate." : "Hierarchy always covers the whole snapshot."}</small>
                </div>
                <label>
                  <span>Subscription</span>
                  <select
                    value={selectedGroup?.subscriptionId ?? subscriptionId}
                    disabled={!scopeSupported || Boolean(selectedGroup)}
                    onChange={(event) => {
                      setSubscriptionId(event.target.value);
                      setResourceGroupId("");
                      invalidateRun();
                    }}
                  >
                    <option value="">All subscriptions</option>
                    {estate.subscriptions.map((subscription) => (
                      <option key={subscription.id} value={subscription.id}>{subscription.displayName}</option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>Resource group</span>
                  <select
                    value={resourceGroupId}
                    disabled={!scopeSupported}
                    onChange={(event) => {
                      setResourceGroupId(event.target.value);
                      invalidateRun();
                    }}
                  >
                    <option value="">All resource groups</option>
                    {resourceGroups.map((group) => (
                      <option key={group.id} value={group.id}>{group.name}</option>
                    ))}
                  </select>
                </label>
              </div>
            </section>
          )}
        </div>

        <aside className="export-dispatch" aria-live="polite">
          <div className="export-dispatch-heading">
            <FileOutput size={20} />
            <div><h2>Export run</h2><p>{kind === "reports" ? "Report package" : "Diagram package"}</p></div>
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
            <div><dt>Selection</dt><dd>{selectedFormats.length} format{selectedFormats.length === 1 ? "" : "s"}</dd></div>
            <div><dt>Access</dt><dd>SQLite only</dd></div>
          </dl>

          <button className="export-run-button" onClick={() => void runExport()} disabled={!canExport}>
            {running ? <LoaderCircle className="spin" size={16} /> : <FileOutput size={16} />}
            {running ? "Generating export" : isTauri ? "Export snapshot" : "Preview export"}
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
