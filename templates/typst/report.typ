// azdocs PDF report. Data arrives as JSON strings via sys.inputs (report,
// branding, diagrams); diagram SVGs are virtual files under /diagrams/ and
// the optional logo is a virtual file whose path arrives as sys.inputs.logo.

#let report = json(bytes(sys.inputs.report))
#let branding = json(bytes(sys.inputs.branding))
#let diagrams = json(bytes(sys.inputs.diagrams))
#let logo-path = sys.inputs.at("logo", default: "")

#let primary = rgb(branding.primary_color)
#let accent = rgb(branding.accent_color)
#let muted = luma(40%)
#let sev-colors = (
  high: rgb("#d13438"),
  medium: rgb("#ff8c00"),
  low: rgb("#c9a900"),
  info: primary,
)

// Stringify any JSON value for a table cell.
#let cell(v) = {
  if v == none { "" } else if type(v) == str { v } else if type(v) == bool {
    if v { "true" } else { "false" }
  } else if type(v) == int or type(v) == float { str(v) } else { repr(v) }
}

// First ~6 useful columns of a query section; the raw ARM id is dropped
// because it never fits a printed page (CSV/XLSX keep the full data).
#let page-columns(columns) = {
  let cols = columns.filter(c => c != "id")
  cols.slice(0, calc.min(6, cols.len()))
}

#let capitalise(s) = if s.len() == 0 { s } else { upper(s.first()) + s.slice(1) }

#let severity-cell(severity) = table.cell(
  fill: sev-colors.at(severity, default: luma(50%)).lighten(75%),
)[#severity]

#let data-table(columns, rows) = table(
  columns: columns.len(),
  inset: 4pt,
  stroke: 0.4pt + luma(75%),
  table.header(..columns.map(c => text(size: 8pt, weight: "bold", c))),
  ..rows
    .map(row => columns.map(c => text(size: 7.5pt, cell(row.at(c, default: none)))))
    .flatten(),
)

#let stat(value, label) = block(
  stroke: 0.5pt + luma(70%),
  radius: 4pt,
  inset: 8pt,
  width: 100%,
)[
  #text(size: 16pt, weight: "bold")[#cell(value)] \
  #text(size: 8pt, fill: muted)[#label]
]

#set document(
  title: branding.title,
  author: if branding.company == "" { () } else { (branding.company,) },
)
#set text(font: "Libertinus Serif", size: 10pt)
#show raw: set text(font: "DejaVu Sans Mono", size: 8.5pt)
#set page(paper: branding.page_size, margin: eval(branding.margin))
#show heading.where(level: 1): it => {
  set text(fill: primary, size: 16pt)
  block(above: 1.6em, below: 0.8em, it)
}
#show heading.where(level: 2): it => {
  set text(fill: primary.darken(15%), size: 12pt)
  block(above: 1.4em, below: 0.6em, it)
}

// ---------------------------------------------------------------- cover ----
#page(footer: none)[
  #v(2fr)
  #align(center)[
    #if logo-path != "" {
      image(logo-path, height: 3cm)
      v(1cm)
    }
    #text(size: 28pt, weight: "bold", fill: primary)[#branding.title]
    #if branding.subtitle != "" [
      \ #text(size: 14pt, fill: muted)[#branding.subtitle]
    ]
    #if branding.company != "" [
      \ #v(0.4cm) #text(size: 12pt)[#branding.company]
    ]
  ]
  #v(1fr)
  #align(center)[
    #text(size: 9pt, fill: muted)[
      Tenant #raw(report.tenant_id) · snapshot #raw(report.snapshot_id) \
      Collected #report.created_at · status #report.status
    ]
  ]
  #v(1fr)
]

// Footer with page numbers on every page after the cover.
#set page(footer: context {
  let left = branding.footer + (
    if branding.company != "" { " · " + branding.company } else { "" }
  )
  grid(
    columns: (1fr, auto),
    text(size: 8pt, fill: muted, left),
    text(size: 8pt, fill: muted, counter(page).display("1 / 1", both: true)),
  )
})

#outline(title: "Contents", depth: 2)
#pagebreak()

// ------------------------------------------------------------- summary ----
= Executive Summary

#grid(
  columns: (1fr, 1fr, 1fr, 1fr, 1fr),
  gutter: 8pt,
  stat(report.totals.subscriptions, "Subscriptions"),
  stat(report.totals.resource_groups, "Resource groups"),
  stat(report.totals.resources, "Resources"),
  stat(report.totals.findings, "Findings"),
  stat(str(report.tag_coverage.percent) + "%", "Tag coverage"),
)

#let sev = report.severity_counts
Findings by severity: #text(fill: sev-colors.high, weight: "bold")[#sev.high high],
#text(fill: sev-colors.medium, weight: "bold")[#sev.medium medium],
#text(fill: sev-colors.low, weight: "bold")[#sev.low low],
#text(fill: sev-colors.info, weight: "bold")[#sev.info info].
#cell(report.tag_coverage.tagged) resources are tagged and
#cell(report.tag_coverage.untagged) are untagged.

== Resources by type

#data-table(
  ("display", "azure_type", "count").map(c => c),
  report.type_counts,
)

// ------------------------------------------------------------ findings ----
= Findings

#if report.findings.len() == 0 [
  No findings.
] else [
  #table(
    columns: (auto, 1fr, auto, auto),
    inset: 5pt,
    stroke: 0.4pt + luma(75%),
    table.header(
      text(size: 8pt, weight: "bold")[Severity],
      text(size: 8pt, weight: "bold")[Title],
      text(size: 8pt, weight: "bold")[Category],
      text(size: 8pt, weight: "bold")[Check],
    ),
    ..report.findings
      .map(f => (
        severity-cell(f.severity),
        text(size: 8pt, f.title),
        text(size: 8pt, f.category),
        text(size: 7.5pt, font: "DejaVu Sans Mono", f.query_name),
      ))
      .flatten()
  )
]

// ----------------------------------------------------------- inventory ----
#for category in report.categories {
  heading(level: 1, capitalise(category.name))
  for q in category.queries {
    heading(level: 2, q.name)
    if q.description != "" {
      block(text(size: 9pt, fill: muted, q.description))
    }
    data-table(page-columns(q.columns), q.rows)
  }
}

// ------------------------------------------------------- subscriptions ----
= Subscriptions

#for sub in report.subscriptions {
  heading(level: 2, sub.display_name)
  block(text(size: 9pt, fill: muted)[
    #raw(sub.subscription_id) · #cell(sub.resource_count) resources
  ])
  for rg in sub.resource_groups {
    if rg.resources.len() == 0 { continue }
    block(text(size: 10pt, weight: "bold")[
      #rg.name#if rg.location != none [ (#rg.location)]
    ])
    data-table(
      ("name", "display_type", "location", "tags"),
      rg.resources,
    )
  }
}

// ------------------------------------------------------------ diagrams ----
#if diagrams.len() > 0 {
  heading(level: 1, "Diagrams")
  for d in diagrams {
    figure(
      image("/diagrams/" + d.slug + ".svg", width: 100%),
      caption: text(size: 9pt, d.title),
    )
  }
}
