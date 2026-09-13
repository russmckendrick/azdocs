# Reports

```sh
azdocs report --format md|html|csv|xlsx|pdf|docx|all [--include-reference] [--theme <name>] [--snapshot <id|latest>] [--out <dir>]
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

## The PDF and DOCX

The default export is an assessment: architecture, recurring findings, selected
configuration comparisons and traceable review actions. Both formats consume
one shared `PrintDocument`, built from `ReportContext.analysis` and the stored
snapshot. The main report contains:

1. Executive assessment: observations, significance and first decisions.
2. Estate composition: subscriptions, service families and regions.
3. Architecture and dependencies: observed connections, shared targets and
   focused figures.
4. Subscription profiles and up to three distinct group studies per subscription.
5. Security and data protection: one explanation per recorded check.
6. Governance and operational evidence, including limitations of collection.
7. Prioritised review actions linked to the relevant check assessment.
8. Collection coverage: successful, successfully empty, failed and unknown query
   outcomes, plus snapshot provenance.

Group studies are selected by recorded finding priority, cross-group connection
count, then population. Each selection excludes earlier choices and resolves
remaining ties by name and ID. Names are identifiers, never evidence of business
purpose. Finding occurrences, unique resource IDs and unresolved references are
counted separately. Historical severities are retained; explanatory wording does
not reclassify findings. Custom checks keep their original evidence and receive
generic verification guidance.

The intended depth is roughly 30–50 pages for a 307-resource estate, rather than
a fixed quota. Smaller estates are shorter. Significant evidence is not discarded
to meet a page count.

### Optional technical reference

```sh
azdocs report --format pdf --include-reference
azdocs report --format docx --include-reference
azdocs report --format all --include-reference
```

The option defaults to false. It keeps `report.pdf` / `report.docx` as the main
assessment and additionally writes `technical-reference.pdf` and/or
`technical-reference.docx` for the selected print formats. The desktop offers an
unchecked **Include technical reference** control for PDF and Word exports.

The reference uses a smaller working type scale, bordered settings and evidence
tables, and Azure service icons beside resource names. Resource names in the
type index link to their detailed entries; relationship-table endpoints and
finding resource names link to the same entries whenever the resource is present.
Links use normalized ARM IDs, so duplicate names remain distinct. Main-report evidence
examples use bullet points; the explanatory sections retain the assessment's
regular body type.

Each resource shows its name, service type and selected operational settings:
location, SKU, managed identity, service size and state, access controls,
networking, retention and ownership tags where recorded. Disks, subnets, node
pools and similar collections are counted instead of expanded. Settings use
plain-language labels. Missing settings are omitted, never treated as a passed
control. Definitions, scripts, provider capability lists, histories and hidden
tags stay out of print. Individual values are abbreviated after 160 characters,
tag lists after 200; standalone URLs remain complete.

Website screenshots form a gallery followed by a separate capture-details table for each website.
External links show their complete URLs and wrap within the table cells.

The companion retains the complete resource register and subscription → resource
group → resource navigation. Findings are grouped by check, resource and recorded
severity, with counts covering every occurrence. A brief collection summary distinguishes successful, empty, failed and unknown
outcomes; failed queries retain their diagnostic messages.

There is no Evidence appendix in either the assessment or the technical
reference. Raw query rows, policy and role catalogues, original finding titles
and evidence are not printed in the reference. The snapshot database retains the full stored values; use the
Data Workbook, CSV or HTML for data-oriented review of their included fields.
This keeps the reference useful for operational review without turning it into
a dump of Azure provider metadata.

### Diagrams and pagination

Main-report figures show selected relationships, fold known attachments, retain
explicit counts, and split complex sets into smaller figures. Selection creates
`EstateGraph`s; the shared page, layout, connector, icon, SVG and PNG code renders
them. There is no separate report drawing engine. Only diagrams used by the
selected document are generated. The main report has no tall estate hierarchy
picture or repeated per-resource diagrams. See [Diagram standards](../reference/diagrams.md).

Both formats have a cover, contents and internal navigation. Word requests a
field refresh for its contents and page numbers. If the editor suppresses this,
select the contents and choose **Update Field**. Native pagination can differ:
Typst embeds fonts; Word uses locally installed fonts and keeps the document
editable. Both repeat comparison-table headers; the Word package adds the
standard OOXML `w:tblHeader` flag where the library lacks an API.

Tag presence is separate from required-tag compliance. The application's 60%
tag-presence threshold is a descriptive indicator, not a compliance standard.
Where historical required-tag scope was not recorded, a zero finding count is
reported as unavailable audit scope. Monitoring and recovery resources show
observed infrastructure; their existence does not establish delivery, health,
successful backups or tested recovery.

## Document design

Every styled export ships in the same **Field Report** language as the desktop:
paper and ink, regular serif headings, 11-point sans-serif body text, hairline
tables, quiet evidence fills, and
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


## Operational evidence and provenance

Operational and compliance summaries are computed once from the snapshot and
shared by Markdown, HTML/site, XLSX, PDF and DOCX. They distinguish source states,
missing evaluations and evidence age without requesting new Azure data.

Exports do not create a separate `query-provenance.json` file. Exact executed
queries and settings, including failed attempts, remain in the snapshot database.
HTML exposes expandable query definitions; Markdown/site adds a query-provenance
page; XLSX adds a provenance sheet. The optional PDF/DOCX technical reference
summarises collection outcomes without exporting query definitions.
Old snapshots do not acquire invented provenance from today's query pack.

See [operational evidence](../reference/operational-evidence.md) for Microsoft
sources, retention windows and review thresholds.

Next: [Diagrams](diagrams.md)
