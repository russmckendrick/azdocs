# Reports

```sh
azdocs report --format md|html|csv|xlsx|pdf|docx|all [--theme <name>] [--snapshot <id|latest>] [--out <dir>]
```

Reports are generated from the stored snapshot — no network access, no
external tools (PDF and DOCX are rendered natively; no Chromium, no Pandoc).
Everything lands under `./output/` by default:

| Format | Output | Contents |
|---|---|---|
| `md` | `output/docs/` | Markdown docs tree (works in any Git host or wiki) |
| `html` | `output/report.html` + `output/docs-html/` | Single self-contained report **and** a multi-page HTML site |
| `csv` | `output/inventory.csv`, `output/findings.csv` | Flat exports |
| `xlsx` | `output/azdocs.xlsx` | Summary, Inventory (autofilter), Findings (severity colours), Governance (tag coverage and the least compliant groups), one sheet per category |
| `pdf` | `output/report.pdf` | Print-ready document — see below |
| `docx` | `output/report.docx` | The same document, editable in Word |

Wide evidence tables in the PDF and DOCX are trimmed to page width (~6
columns, no raw ARM ids). The full data always lives in the CSV/XLSX/HTML
outputs.

## The PDF and DOCX

Both consume the same internal `PrintDocument` and the same Field Report
[theme](../reference/themes.md), so they are one document in two containers.
Content, order, heading levels, table labels and values, captions, icons and
diagram selection are composed once:

1. **Cover** — logo, title, subtitle, company, snapshot metadata.
2. **Contents** — a real TOC. The DOCX marks it for refresh so Word recalculates
   its page numbers after laying out the document. If an editor or security
   policy suppresses automatic field updates, select the TOC and choose
   **Update Field** (or press `Ctrl+A`, then `F9` in desktop Word).
3. **Executive summary** — KPI figures, a short snapshot narrative, the largest
   resource types and locations, and the highest-priority findings.
4. **Estate overview** — hierarchy and network diagrams early in the report so
   they orient the detail that follows. Each is emitted at a fixed share of an
   A4 portrait page (quarter, third, half or full) and flows, so several can
   tile onto one sheet. Resources are aggregated by type (`Storage Account
   ×13`) to stay readable; run `azdocs diagram` for full per-resource detail.
5. **Findings** — every finding as a flowing callout, grouped high to
   informational rather than packed into one table.
6. **Governance** — tag coverage by key and by subscription, each judged
   against the healthy threshold, then the least compliant resource groups from
   the `missing_required_tags` audit with the groups past the flagged share
   named. This is the same analysis the desktop explorer's Governance workspace
   draws, from the same Rust code, so the two never disagree about an estate.
7. **Resources by type** — a lightweight index: every resource of each type
   with its
   subscription, group and location. The body below is grouped the way Azure
   is, which scatters one type across many groups; this restores the
   compliance sweep ("every storage account") without repeating the detail.
   Each type is a level-2 heading with its Azure icon.
8. **The estate** — laid out as Azure itself is: **subscription → resource
   group → resource**. Each group opens with its own summarised diagram, then
   documents every resource inside it: a name plate, a **relationship diagram**
   of the resource and everything attached to it, a definition list of
   settings flattened from its properties, any findings raised against it,
   and its related resources. Per-group diagrams are capped at 60.
9. **Evidence appendix** — the collected inventory-query results. A compact
   one-row result becomes a definition list; larger result sets remain tables
   because row-to-row comparison is the useful reading mode there.

See [Diagram standards](../reference/diagrams.md) for the report and CLI
diagram contracts.

Resources with no relationships get no diagram — a lone box says nothing the
settings list does not. Per-resource diagrams are capped at 250 for a single
report; passing the cap is logged.

Pagination can differ because Typst lays out a fixed print document while Word
keeps the DOCX editable and reflows it using locally installed fonts. Word also
cannot repeat table headers with the DOCX library used here and approximates a
full-bleed block cover inside the printable page area. Those native constraints
do not change the report's content or hierarchy.

## Document design

Every styled export ships in the same **Field Report** language as the desktop:
paper and ink, serif-led headings, hairline tables, quiet evidence fills, and
colour reserved for branding and signals. `[branding]` still supplies the
company name, title, brand colour, logo and footer.

`field-report` is the only built-in theme. Themes remain TOML data so an
organisation can supply a custom document system, or select it for one CLI run
with `--theme <name>`. See [reference/themes.md](../reference/themes.md).

## The docs tree

```
output/docs/
├── index.md                     # estate summary: counts, types, locations, tag coverage
├── findings.md                  # all findings grouped by severity
├── networking.md  compute.md  … # per-category query result tables
├── subscriptions/<name>.md      # per-subscription inventory by resource group
└── resources/<sub>/<rg>.md      # per-resource detail pages
```

The **detail pages** are the deep end: one section per resource with settings
flattened from its properties, warning callouts for findings on that resource,
and related-resource links derived from the relationship edges. The Markdown
and HTML docs tree keeps its tabular settings presentation; the print formats
use the flowing definition-list treatment described above.

## The HTML report

`output/report.html` is one self-contained file (inline CSS/JS, dark-mode
aware) with severity badges and client-side table filtering — suitable for
email or SharePoint. `output/docs-html/` is the docs tree as static HTML with
navigation.

Next: [Diagrams](diagrams.md)
