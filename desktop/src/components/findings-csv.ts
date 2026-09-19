import type { Finding } from "../types";

function csvCell(value: string) {
  return /[",\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value;
}

/** The same columns as findings.csv from the CLI, so the two files match. */
export function findingsCsv(findings: Finding[], columns: string[]) {
  return (
    [
      columns.map(csvCell).join(","),
      ...findings.map((finding) =>
        [finding.severity, finding.category, finding.queryName, finding.title, finding.resourceId ?? ""]
          .map(csvCell)
          .join(","),
      ),
    ].join("\n") + "\n"
  );
}
