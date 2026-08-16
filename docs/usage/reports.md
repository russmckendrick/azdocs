# Reports

```sh
azdocs report --format md|html|csv|xlsx|pdf|docx|all  [--snapshot <id|latest>] [--out <dir>]
```

Reports are generated from the stored snapshot — no network access, no
external tools (PDF and DOCX are rendered natively; no Chromium, no Pandoc).
Everything lands under `./output/` by default:

| Format | Output | Contents |
|---|---|---|
| `md` | `output/docs/` | Markdown docs tree (works in any Git host or wiki) |
| `html` | `output/report.html` + `output/docs-html/` | Single self-contained report **and** a multi-page HTML site |
| `csv` | `output/inventory.csv`, `output/findings.csv` | Flat exports |
| `xlsx` | `output/azdocs.xlsx` | Summary, Inventory (autofilter), Findings (severity colours), one sheet per category |
| `pdf` | `output/report.pdf` | Print-ready document: cover, TOC, executive summary, findings, category tables, network/hierarchy diagrams |
| `docx` | `output/report.docx` | The same structure as the PDF (minus diagrams), editable in Word |

PDF and DOCX tables are trimmed to page width (~6 columns, no raw ARM ids) —
the full data always lives in the CSV/XLSX/HTML outputs. All formats pick up
the [`[branding]` config](configuration.md#branding): company name, title,
colors, logo, and footer; HTML and the docs site are themed by it too.

## The docs tree

```
output/docs/
├── index.md                     # estate summary: counts, types, locations, tag coverage
├── findings.md                  # all findings grouped by severity
├── networking.md  compute.md  … # per-category query result tables
├── subscriptions/<name>.md      # per-subscription inventory by resource group
└── resources/<sub>/<rg>.md      # per-resource detail pages
```

The **detail pages** are the deep end: one section per resource with a
settings table flattened from its properties, warning callouts for findings on
that resource, and related-resource links derived from the relationship edges.

## The HTML report

`output/report.html` is one self-contained file (inline CSS/JS, dark-mode
aware) with severity badges and client-side table filtering — suitable for
email or SharePoint. `output/docs-html/` is the docs tree as static HTML with
navigation.

Next: [Diagrams](diagrams.md)
