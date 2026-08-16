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
| `pdf` | `output/report.pdf` | Print-ready document — see below |
| `docx` | `output/report.docx` | The same document, editable in Word |

The summary tables in the PDF and DOCX are trimmed to page width (~6 columns,
no raw ARM ids) — the full data always lives in the CSV/XLSX/HTML outputs.

## The PDF and DOCX

Both are built from the same content and the same
[theme](../reference/themes.md), so they are one document in two containers:

1. **Cover** — logo, title, subtitle, company, snapshot metadata.
2. **Contents** — a real TOC; in Word it is a field, so it renumbers on edit.
3. **Executive summary** — KPI figures, severity breakdown, resources by type.
4. **Findings** — every finding, colour-coded by severity.
5. **Category tables** — the inventory query results, trimmed to page width.
6. **One chapter per resource type** — the detail. Each chapter opens with the
   type's Azure icon, then documents every resource of that type: a name
   plate, a **relationship diagram** of the resource and everything attached
   to it, a **settings table** flattened from its properties, any findings
   raised against it, and its related resources.
7. **Subscriptions** — inventory by subscription and resource group.
8. **Diagrams** — estate hierarchy, network topology, and one summarised
   diagram per resource group. Each is emitted at a fixed share of an A4
   portrait page (quarter, third, half or full) and flows, so several tile onto
   one sheet. Resources are aggregated by type (`Storage Account ×13`) to stay
   readable; run `azdocs diagram` for the full per-resource detail. Per-group
   diagrams are capped at 60. See
   [Diagram standards](../reference/diagrams.md).

Resources with no relationships get no diagram — a lone box says nothing the
settings table does not. Per-resource diagrams are capped at 250 for a single
report; passing the cap is logged.

## Themes

`[branding] theme` picks the look. All formats pick up the
[`[branding]` config](configuration.md#branding) — company name, title,
colours, logo and footer — and the theme derives its palette from your two
brand colours.

| Theme | Look |
|---|---|
| `fluent` | Azure-native: colour band cover, filled table headers, zebra rows |
| `editorial` | Consultancy report: centred cover, chapter divider pages, hairline tables |
| `dashboard` | Modern tech: colour block cover, tinted KPI cards, banded tables |

Themes are TOML files, not code — write your own with
[reference/themes.md](../reference/themes.md).

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
