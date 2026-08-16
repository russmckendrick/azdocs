// azdocs PDF report: content and document structure only. Every design value
// lives in theme.typ, driven by the resolved theme in sys.inputs.theme.
//
// Data arrives as JSON strings via sys.inputs (report, branding, theme,
// diagrams); diagram SVGs are virtual files under /diagrams/ and the optional
// logo is a virtual file whose path arrives as sys.inputs.logo.

#import "/theme.typ": *

#let report = json(bytes(sys.inputs.report))
#let branding = json(bytes(sys.inputs.branding))
#let diagrams = json(bytes(sys.inputs.diagrams))
#let logo-path = sys.inputs.at("logo", default: "")

#set document(
  title: branding.title,
  author: if branding.company == "" { () } else { (branding.company,) },
)

#show: body => setup(branding, body)

// ---------------------------------------------------------------- cover ----
#cover(report, branding, logo-path)

// Chrome starts after the cover, which sets its own bare page.
#set page(
  header: running-header(branding),
  footer: running-footer(branding),
)

#outline(title: "Contents", depth: 2)
#pagebreak()

// -------------------------------------------------------------- summary ----
#chapter("Executive Summary")

#stat-row((
  (report.totals.subscriptions, "Subscriptions"),
  (report.totals.resource_groups, "Resource groups"),
  (report.totals.resources, "Resources"),
  (report.totals.findings, "Findings"),
  (str(report.tag_coverage.percent) + "%", "Tag coverage"),
))

#v(0.8em)

#let sv = report.severity_counts
Findings by severity:
#text(fill: sev("high", "text"), weight: "bold")[#sv.high high],
#text(fill: sev("medium", "text"), weight: "bold")[#sv.medium medium],
#text(fill: sev("low", "text"), weight: "bold")[#sv.low low],
#text(fill: sev("info", "text"), weight: "bold")[#sv.info info].
#cell(report.tag_coverage.tagged) resources are tagged and
#cell(report.tag_coverage.untagged) are untagged.

== Resources by type

#data-table(
  ("display", "azure_type", "count"),
  report.type_counts,
  widths: (auto, 1fr, auto),
)

// ------------------------------------------------------------- findings ----
#chapter("Findings")

#if report.findings.len() == 0 [
  No findings.
] else {
  table(
    columns: (auto, 1fr, auto, auto),
    inset: lay.table_inset_pt * 1pt,
    stroke: table-stroke,
    fill: row-fill,
    table.header(
      repeat: true,
      header-cell("Severity"),
      header-cell("Title"),
      header-cell("Category"),
      header-cell("Check"),
    ),
    ..report.findings
      .map(f => (
        severity-cell(f.severity),
        text(size: typ.table_pt * 1pt, breakable(f.title)),
        text(size: typ.table_pt * 1pt, f.category),
        mono(breakable(f.query_name), size: typ.table_pt * 1pt),
      ))
      .flatten()
  )
}

// ------------------------------------------------------------ inventory ----
#for category in report.categories {
  chapter(capitalise(category.name))
  for q in category.queries {
    heading(level: 2, q.name)
    if q.description != "" {
      block(text(size: typ.small_pt * 1pt, fill: muted, q.description))
    }
    // print_columns is computed once in Rust so the DOCX emitter trims to
    // exactly the same set.
    data-table(q.print_columns, q.rows)
  }
}

// -------------------------------------------------------- subscriptions ----
#chapter("Subscriptions")

#for sub in report.subscriptions {
  heading(level: 2, sub.display_name)
  block(text(size: typ.small_pt * 1pt, fill: muted)[
    #mono(breakable(sub.subscription_id)) · #cell(sub.resource_count) resources
  ])
  for rg in sub.resource_groups {
    if rg.resources.len() == 0 { continue }
    heading(level: 3, rg.name + if rg.location != none { " (" + rg.location + ")" } else { "" })
    data-table(
      ("name", "display_type", "location", "tags"),
      rg.resources,
      widths: (auto, auto, auto, 1fr),
    )
  }
}

// ------------------------------------------------------------- diagrams ----
#if diagrams.len() > 0 {
  // Estate diagrams are far wider than they are tall; a portrait page shrinks
  // them past readability, so the chapter runs landscape.
  set page(flipped: true)
  chapter("Diagrams")
  for d in diagrams {
    figure(
      block(width: 100%, height: 78%, image("/diagrams/" + d.slug + ".svg", fit: "contain")),
      caption: text(size: typ.small_pt * 1pt, d.title),
    )
    pagebreak(weak: true)
  }
}
