import type { ExportKind } from "../types";

export type ExportPresetId = "field-report" | "word-report" | "data-workbook" | "diagram-workbook";

/**
 * What each deliverable produces. The copy (label, action, detail, includes)
 * lives in `desktop.exports.presets` under the same ids; only what names a
 * file or a CLI value stays here.
 */
interface ExportPreset {
  id: ExportPresetId;
  extension: string;
  exportKind: ExportKind;
  formats: string[];
  diagramType?: string;
}

export const EXPORT_PRESETS: ExportPreset[] = [
  { id: "field-report", extension: ".pdf", exportKind: "reports", formats: ["pdf"] },
  { id: "word-report", extension: ".docx", exportKind: "reports", formats: ["docx"] },
  { id: "data-workbook", extension: ".xlsx", exportKind: "reports", formats: ["xlsx"] },
  {
    id: "diagram-workbook",
    extension: ".drawio",
    exportKind: "diagrams",
    formats: ["drawio"],
    diagramType: "workbook",
  },
];

