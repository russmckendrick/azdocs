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
#let resource-diagrams = json(bytes(sys.inputs.resource_diagrams))
#let group-diagrams = json(bytes(sys.inputs.group_diagrams))
#let icons = json(bytes(sys.inputs.icons))
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

// ------------------------------------------------------- type index ----
// The by-group body below is the right shape for reading an estate, but it
// scatters resources of one type across many groups. This index restores the
// compliance sweep — "every storage account" — without duplicating detail.
#chapter("Resources by type")

#for section in report.resource_types {
  heading(level: 2, section.display)
  data-table(
    ("name", "subscription_name", "resource_group", "location"),
    section.resources,
    widths: (auto, auto, auto, 1fr),
  )
}

// ------------------------------------------------------ resource detail ----
// Laid out the way Azure itself is: subscription, then resource group, then
// the resources inside it. The group's diagram heads its section, so the
// picture and the configuration it describes sit together.
#for sub in report.subscriptions {
  chapter(sub.display_name)
  block(text(size: typ.small_pt * 1pt, fill: muted)[
    #mono(breakable(sub.subscription_id)) · #cell(sub.resource_count) resources
  ])

  for page in report.details {
    if page.subscription_name != sub.display_name { continue }
    heading(level: 2, page.resource_group)
    block(text(size: typ.small_pt * 1pt, fill: muted)[
      #cell(page.resources.len()) resources
      #if page.location != none [ · #page.location]
    ])

    let group-slug = group-diagrams.at(lower(page.group_key), default: "")
    if group-slug != "" {
      figure(
        image("/diagrams/" + group-slug + ".svg", width: 100%),
        caption: text(size: typ.small_pt * 1pt, page.resource_group),
      )
    }

    for detail in page.resources {
      let slug = resource-diagrams.at(lower(detail.arm_id), default: "")
      resource-detail(
        detail,
        if slug == "" { "" } else { "/diagrams/" + slug + ".svg" },
      )
    }
  }
}

// ------------------------------------------------------------- diagrams ----
#if diagrams.len() > 0 {
  // Each diagram is emitted at a fixed share of an A4 portrait page (see
  // `diagram::page`), so they are placed at their natural size and allowed to
  // flow: two halves or three thirds share a sheet. Forcing a page break per
  // diagram — or running the chapter landscape, as it used to — throws that away.
  chapter("Diagrams")
  for d in diagrams {
    figure(
      image("/diagrams/" + d.slug + ".svg", width: 100%),
      caption: text(size: typ.small_pt * 1pt, d.title),
    )
    v(typ.base_pt * 0.8pt)
  }
}
