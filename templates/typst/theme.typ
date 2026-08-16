// Design system for the PDF report. Every value comes from the resolved theme
// (`sys.inputs.theme`, produced by src/report/theme.rs); this file only
// implements the closed set of layout strategies a theme file can choose
// between. Nothing here knows a theme's name.

#let theme = json(bytes(sys.inputs.theme))

#let pal = theme.palette
#let typ = theme.typography
#let lay = theme.layout

#let primary = rgb(pal.primary)
#let primary-dark = rgb(pal.primary_dark)
#let primary-tint = rgb(pal.primary_tint)
#let accent = rgb(pal.accent)
#let accent-tint = rgb(pal.accent_tint)
#let on-primary = rgb(pal.on_primary)
#let band = rgb(pal.band)
#let on-band = rgb(pal.on_band)
#let ink = rgb(pal.ink)
#let muted = rgb(pal.muted)
#let rule = rgb(pal.rule)
#let zebra = rgb(pal.zebra)

#let rule-stroke = lay.rule_pt * 1pt + rule
#let radius = lay.radius_pt * 1pt

#let sev(severity, part) = {
  let colors = pal.severity.at(severity, default: pal.severity.info)
  rgb(colors.at(part))
}

// ------------------------------------------------------------- text bits ----

// Stringify any JSON value for a table cell.
#let cell(v) = {
  if v == none { "" } else if type(v) == str { v } else if type(v) == bool {
    if v { "true" } else { "false" }
  } else if type(v) == int or type(v) == float { str(v) } else { repr(v) }
}

// ARM ids, URLs and resource names have no spaces, so a long one would push a
// table column past the page edge. Offering a zero-width break after each
// separator lets them wrap inside the cell instead.
#let breakable(s) = {
  let out = ""
  for ch in s {
    out += ch
    if ch == "/" or ch == "-" or ch == "." or ch == "_" { out += "\u{200B}" }
  }
  out
}

#let capitalise(s) = if s.len() == 0 { s } else { upper(s.first()) + s.slice(1) }

#let mono(body, size: none) = text(
  font: typ.mono,
  size: if size == none { typ.small_pt * 1pt } else { size },
  body,
)

// ---------------------------------------------------------------- tables ----

// Header cell styling per table strategy.
#let header-cell(label) = {
  let content = text(
    size: typ.table_header_pt * 1pt,
    weight: "semibold",
    fill: if lay.table == "solid-header" { on-primary } else { ink },
    label,
  )
  if lay.table == "solid-header" {
    table.cell(fill: primary, content)
  } else if lay.table == "banded" {
    table.cell(fill: primary-tint, content)
  } else {
    table.cell(content)
  }
}

#let table-stroke = if lay.table == "solid-header" {
  rule-stroke
} else {
  // Hairline and banded rule horizontally only; vertical rules add noise to
  // dense inventory tables without helping the reader.
  (x, y) => (
    top: if y == 0 { rule-stroke } else { lay.rule_pt * 1pt + rule.lighten(30%) },
    bottom: rule-stroke,
    left: none,
    right: none,
  )
}

#let row-fill(_, y) = if lay.zebra_rows and calc.odd(y) { zebra } else { none }

/// A data table with a repeating header, wrapped long values and an explicit
/// empty state. `widths` may be none to size every column automatically.
#let data-table(columns, rows, widths: none) = {
  if columns.len() == 0 or rows.len() == 0 {
    return block(text(size: typ.small_pt * 1pt, fill: muted, style: "italic")[No results.])
  }
  block(breakable: true, table(
    columns: if widths == none { columns.map(_ => auto) } else { widths },
    inset: lay.table_inset_pt * 1pt,
    stroke: table-stroke,
    fill: row-fill,
    // repeat: true keeps the header on every page a long table spans.
    table.header(repeat: true, ..columns.map(header-cell)),
    ..rows
      .map(row => columns.map(c => text(
        size: typ.table_pt * 1pt,
        breakable(cell(row.at(c, default: none))),
      )))
      .flatten(),
  ))
}

#let severity-cell(severity) = table.cell(fill: sev(severity, "fill"))[
  #text(size: typ.table_pt * 1pt, weight: "semibold", fill: sev(severity, "text"), severity)
]

// ----------------------------------------------------------------- stats ----

#let stat(value, label) = {
  let body = [
    #text(size: typ.stat_value_pt * 1pt, weight: "bold", fill: primary-dark)[#cell(value)] \
    #text(size: typ.stat_label_pt * 1pt, fill: muted)[#label]
  ]
  if lay.stat == "card" {
    block(
      fill: primary-tint,
      stroke: (top: 2pt + accent),
      radius: radius,
      inset: 8pt,
      width: 100%,
      body,
    )
  } else if lay.stat == "outline" {
    block(stroke: rule-stroke, radius: radius, inset: 8pt, width: 100%, body)
  } else {
    block(inset: (y: 4pt), width: 100%, body)
  }
}

#let stat-row(entries) = grid(
  columns: entries.map(_ => 1fr),
  gutter: 8pt,
  ..entries.map(e => stat(e.at(0), e.at(1))),
)

// ----------------------------------------------------------------- cover ----

#let cover-meta(report) = text(size: typ.small_pt * 1pt, fill: muted)[
  Tenant #mono(breakable(report.tenant_id)) · snapshot #mono(breakable(report.snapshot_id)) \
  Collected #report.created_at · status #report.status
]

// Logos are user-supplied at any aspect ratio, so constrain both axes.
#let cover-logo(logo-path, height: 2.4cm) = if logo-path != "" {
  image(logo-path, width: 6cm, height: height, fit: "contain")
}

#let cover(report, branding, logo-path) = {
  if lay.cover == "block" {
    page(footer: none, header: none, margin: 0pt, fill: band)[
      #v(1fr)
      #block(inset: (x: 3cm))[
        #if logo-path != "" [#cover-logo(logo-path) #v(0.8cm)]
        #text(size: typ.title_pt * 1pt, weight: "bold", fill: on-band)[#branding.title]
        #if branding.subtitle != "" [
          \ #v(0.2cm) #text(size: typ.subtitle_pt * 1pt, fill: on-band.transparentize(20%))[#branding.subtitle]
        ]
        #if branding.company != "" [
          \ #v(0.4cm) #text(size: typ.subtitle_pt * 1pt, fill: on-band)[#branding.company]
        ]
      ]
      #v(1fr)
      // The muted grey of cover-meta has no contrast on a saturated band, so
      // the block cover recolours it rather than dimming it further.
      #block(inset: (x: 3cm, bottom: 2.5cm), text(fill: on-band)[
        #text(size: typ.small_pt * 1pt)[
          Tenant #mono(breakable(report.tenant_id)) ·
          snapshot #mono(breakable(report.snapshot_id)) \
          Collected #report.created_at · status #report.status
        ]
      ])
    ]
  } else if lay.cover == "band" {
    page(footer: none, header: none, margin: 0pt)[
      #block(width: 100%, height: lay.cover_band_pt * 1pt, fill: band)
      #block(inset: (x: 2.5cm, top: 3cm))[
        #if logo-path != "" [#cover-logo(logo-path) #v(0.8cm)]
        #text(size: typ.title_pt * 1pt, weight: "bold", fill: primary)[#branding.title]
        #if branding.subtitle != "" [
          \ #v(0.2cm) #text(size: typ.subtitle_pt * 1pt, fill: muted)[#branding.subtitle]
        ]
        #if branding.company != "" [
          \ #v(0.4cm) #text(size: typ.subtitle_pt * 1pt, fill: ink)[#branding.company]
        ]
      ]
      #v(1fr)
      #block(inset: (x: 2.5cm, bottom: 2.5cm), cover-meta(report))
    ]
  } else {
    page(footer: none, header: none)[
      #v(2fr)
      #align(center)[
        #if logo-path != "" [#cover-logo(logo-path) #v(1cm)]
        #text(size: typ.title_pt * 1pt, weight: "semibold", fill: ink)[
          #upper(branding.title)
        ]
        #v(0.7cm)
        #line(length: 40%, stroke: rule-stroke)
        #v(0.7cm)
        #if branding.subtitle != "" [
          #text(size: typ.subtitle_pt * 1pt, fill: muted)[#branding.subtitle]
          #v(0.35cm)
        ]
        #if branding.company != "" [
          #text(size: typ.subtitle_pt * 1pt, fill: ink)[#branding.company]
        ]
      ]
      #v(1fr)
      #align(center, cover-meta(report))
      #v(1fr)
    ]
  }
}

// ------------------------------------------------------ resource detail ----

/// Resource-type chapter opener. This *is* the level-1 heading — a scoped show
/// rule adds the type's Azure icon and rule — so the title appears once and
/// the outline and running header still see it.
///
/// Each type starts a new page whatever the theme: these chapters are long,
/// and running two types together makes the document hard to navigate.
#let type-chapter(title, icon-path) = {
  pagebreak(weak: true)
  [
    #show heading.where(level: 1): it => block(
      width: 100%,
      above: 0em,
      below: 1em,
      // stack, not consecutive blocks: paragraph spacing between the title and
      // its rule would open a gap several times the intended one.
      stack(
        spacing: 6pt,
        grid(
          columns: (auto, 1fr),
          align: horizon,
          gutter: 10pt,
          if icon-path != "" {
            image(icon-path, width: 30pt, height: 30pt, fit: "contain")
          } else { [] },
          text(size: typ.h1_pt * 1pt, weight: "bold", fill: primary, upper(it)),
        ),
        line(length: 100%, stroke: 1.5pt + primary),
      ),
    )
    #heading(level: 1, title)
  ]
}

/// Name plate above each resource's detail, so a reader scanning a long
/// chapter can find one resource without reading the settings tables.
#let resource-plate(name) = block(
  width: 100%,
  fill: if lay.table == "hairline" { none } else { primary },
  stroke: if lay.table == "hairline" { (bottom: 1pt + primary) } else { none },
  radius: if lay.table == "hairline" { 0pt } else { radius },
  inset: (x: 8pt, y: 5pt),
  above: 1.4em,
  below: 0.7em,
  text(
    size: typ.h3_pt * 1pt,
    weight: "bold",
    fill: if lay.table == "hairline" { primary } else { on-primary },
    upper(name),
  ),
)

/// Small labelled rule introducing a sub-block (Settings, Findings, Related).
#let sub-label(title) = block(width: 100%, above: 1em, below: 0.5em)[
  #text(size: typ.small_pt * 1pt, weight: "semibold", fill: primary-dark, title)
  #v(0.15em)
  #line(length: 100%, stroke: rule-stroke)
]

/// Two-column key/value table used for a resource's settings.
#let settings-table(settings) = if settings.len() == 0 {
  block(text(size: typ.small_pt * 1pt, fill: muted, style: "italic")[No settings recorded.])
} else {
  block(breakable: true, table(
    columns: (0.34fr, 0.66fr),
    inset: lay.table_inset_pt * 1pt,
    stroke: table-stroke,
    fill: row-fill,
    ..settings
      .enumerate()
      .map(((i, s)) => (
        text(size: typ.table_pt * 1pt, weight: "semibold", s.key),
        text(size: typ.table_pt * 1pt, breakable(s.value)),
      ))
      .flatten(),
  ))
}

#let callout-list(callouts) = for c in callouts {
  block(
    width: 100%,
    fill: sev(c.severity, "fill"),
    stroke: (left: 2.5pt + sev(c.severity, "text")),
    inset: (x: 7pt, y: 5pt),
    above: 0.4em,
    below: 0.4em,
  )[
    #text(size: typ.table_pt * 1pt, weight: "bold", fill: sev(c.severity, "text"))[
      #upper(c.severity)
    ]
    #h(6pt)
    #text(size: typ.table_pt * 1pt)[#c.title]
  ]
}

/// One resource: name plate, neighbourhood diagram, settings, findings and
/// related resources. Kept in one place so all three themes stay in step.
#let resource-detail(detail, diagram-path) = {
  resource-plate(detail.name)
  block(text(size: typ.small_pt * 1pt, fill: muted)[
    #detail.display_type · #detail.subscription_name
    #if detail.resource_group != none [ · #detail.resource_group]
    #if detail.location != none [ · #detail.location] \
    #mono(breakable(detail.arm_id), size: (typ.small_pt - 0.5) * 1pt)
  ])

  if diagram-path != "" {
    sub-label("Relationships")
    // Fixed height rather than full width: a neighbourhood graph is one short
    // row of nodes, so scaling it to the text width would blow the icons up.
    align(center, block(width: 100%, height: 3.6cm, image(diagram-path, fit: "contain")))
  }

  sub-label("Settings")
  settings-table(detail.settings)

  if detail.findings.len() > 0 {
    sub-label("Findings")
    callout-list(detail.findings)
  }

  if detail.related.len() > 0 {
    sub-label("Related resources")
    block(text(size: typ.table_pt * 1pt, detail.related.join(" · ")))
  }
}

// --------------------------------------------------------------- chrome ----

#let running-header(branding) = context {
  if not lay.running_header { return }
  // Name the chapter this page belongs to: the one starting on the page when
  // there is one, otherwise the one still running from an earlier page.
  // Taking the *last* heading on the page would label a page by a chapter
  // that only begins in its final centimetre.
  let here-page = here().page()
  let chapters = query(heading.where(level: 1))
  let starting = chapters.filter(h => h.location().page() == here-page)
  let earlier = chapters.filter(h => h.location().page() < here-page)
  let title = if starting.len() > 0 {
    starting.first().body
  } else if earlier.len() > 0 {
    earlier.last().body
  } else {
    branding.title
  }
  block(width: 100%, stroke: (bottom: rule-stroke), inset: (bottom: 4pt))[
    #grid(
      columns: (1fr, auto),
      text(size: typ.stat_label_pt * 1pt, fill: muted, title),
      text(size: typ.stat_label_pt * 1pt, fill: muted, branding.title),
    )
  ]
}

#let running-footer(branding) = context {
  let left = branding.footer + (
    if branding.company != "" { " · " + branding.company } else { "" }
  )
  grid(
    columns: (1fr, auto),
    text(size: typ.stat_label_pt * 1pt, fill: muted, left),
    text(
      size: typ.stat_label_pt * 1pt,
      fill: muted,
      counter(page).display("1 / 1", both: true),
    ),
  )
}

/// Start a top-level chapter. Themes with divider pages get the heading alone
/// on its own page; the heading itself moves there rather than being repeated
/// after the divider, so the title appears once and the outline still sees it.
#let chapter(title) = if lay.divider_pages {
  pagebreak(weak: true)
  v(1fr)
  [
    // A divider page is mostly whitespace, so the title is set larger than an
    // in-flow heading of the same level would be.
    #show heading.where(level: 1): it => block(
      width: 100%,
      stroke: (top: 1.5pt + primary, bottom: 1.5pt + primary),
      inset: (y: 16pt),
      text(size: (typ.h1_pt + 10) * 1pt, weight: "bold", fill: primary, it),
    )
    #heading(level: 1, title)
  ]
  v(1.7fr)
  pagebreak()
} else {
  heading(level: 1, title)
  // Themes without a filled table header lean on rules to separate sections.
  if lay.table != "solid-header" {
    v(-0.5em)
    line(length: 100%, stroke: rule-stroke)
  }
}

/// Document-wide rules. Applied once by report.typ via a show rule.
#let setup(branding, body) = {
  set text(font: typ.sans, size: typ.base_pt * 1pt, fill: ink)
  set par(leading: (typ.line_height - 1.0) * typ.base_pt * 1pt, justify: false)
  show raw: set text(font: typ.mono, size: typ.small_pt * 1pt)
  set page(paper: branding.page_size, margin: eval(branding.margin))

  set heading(numbering: if lay.heading_numbering { "1.1" } else { none })
  // Text only. Rules under a level-1 heading belong to whoever opened the
  // chapter (`chapter` / `type-chapter`); a scoped show rule *adds* to this one
  // rather than replacing it, so drawing a rule here would double it up.
  show heading.where(level: 1): it => block(
    above: 1.8em,
    below: 0.9em,
    text(size: typ.h1_pt * 1pt, weight: "bold", fill: primary, it),
  )
  show heading.where(level: 2): it => block(
    above: 1.5em,
    below: 0.6em,
    text(size: typ.h2_pt * 1pt, weight: "semibold", fill: primary-dark, it),
  )
  show heading.where(level: 3): it => block(
    above: 1.2em,
    below: 0.5em,
    text(size: typ.h3_pt * 1pt, weight: "semibold", fill: ink, it),
  )
  show link: set text(fill: accent)

  body
}
