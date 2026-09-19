// Design system for the PDF report. Every value comes from the resolved theme
// (`sys.inputs.theme`, produced by src/report/theme.rs); this file only
// implements the closed set of layout strategies a theme file can choose
// between. Nothing here knows a theme's name.

#let theme = json(bytes(sys.inputs.theme))
#let reference-grid = json(bytes(sys.inputs.document)).technical_reference and theme.layout.reference_table_borders
// Every word this file prints comes from the resolved labels
// (`sys.inputs.labels`, produced by src/labels); nothing here is English.
#let labels = json(bytes(sys.inputs.labels))

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
#let breakable(s, max-span: 12) = {
  let out = ""
  let span = 0
  for ch in s {
    out += ch
    span += 1
    if ch == " " { span = 0 }
    if ch == "/" or ch == "-" or ch == "." or ch == "_" or span >= max-span {
      out += "\u{200B}"
      span = 0
    }
  }
  out
}

#let capitalise(s) = if s.len() == 0 { s } else { upper(s.first()) + s.slice(1) }

#let mono(body, size: none) = text(
  font: typ.mono,
  size: if size == none { typ.small_pt * 1pt } else { size },
  body,
)

#let external-link(url, display-url, size: typ.small_pt * 1pt) = link(url, grid(
  columns: (auto, 1fr),
  column-gutter: 4pt,
  align: top,
  image("/external-link.svg", width: size),
  text(font: typ.sans, size: size, style: "normal", fill: accent, breakable(display-url)),
))

#let display(body, size, weight: "regular", fill: ink) = text(
  font: typ.serif,
  size: size,
  weight: weight,
  fill: fill,
  body,
)

// ---------------------------------------------------------------- tables ----

// Header cell styling per table strategy.
#let header-cell(label, max-span: 12) = {
  let content = text(
    size: typ.table_header_pt * 1pt,
    weight: "semibold",
    fill: if lay.table == "solid-header" { on-primary } else { ink },
    breakable(label, max-span: max-span),
  )
  if lay.table == "solid-header" {
    table.cell(fill: primary, content)
  } else if lay.table == "banded" {
    table.cell(fill: primary-tint, content)
  } else {
    table.cell(content)
  }
}

#let table-stroke = if reference-grid or lay.table == "solid-header" {
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

#let empty-state(body) = block(below: 0.8em,
  text(size: typ.small_pt * 1pt, fill: muted, style: "italic", body),
)

#let table-value(value, mono-value: false, max-span: 12) = if mono-value {
  mono(breakable(value, max-span: max-span), size: typ.table_pt * 1pt)
} else {
  text(
    size: typ.table_pt * 1pt,
    breakable(value, max-span: max-span),
  )
}

/// Render a display-ready semantic table. Labels and JSON value formatting
/// have already been resolved by PrintDocument, so no report logic lives here.
#let print-table(_kind, columns, rows, links, keep-together: false) = {
  if columns.len() == 0 or rows.len() == 0 {
    return empty-state(labels.report.evidence.no_results)
  }
  // Narrow evidence columns must fit even unspaced property names and enums.
  let max-span = if columns.len() >= 4 { 1 } else { 12 }
  block(above: 0.25em, below: 0.85em, breakable: not keep-together, table(
    columns: columns.map(column => column.weight * 1fr),
    inset: lay.table_inset_pt * 1pt,
    stroke: table-stroke,
    fill: row-fill,
    table.header(repeat: true, ..columns.map(column => header-cell(column.label, max-span: max-span))),
    ..rows
      .enumerate().map(((row-index, row)) => row.enumerate().map(((index, value)) => {
        let column = columns.at(index)
        let target = links.find(entry => entry.row == row-index and entry.column == index)
        let content = table-value(value, mono-value: column.mono, max-span: max-span)
        if target == none { content }
        else if target.external { external-link(target.target, value, size: typ.table_pt * 1pt) }
        else { link(label(target.target), text(fill: accent, content)) }
      }))
      .flatten(),
  ))
}

/// A compact definition list for settings and small query results. The
/// semantic document decides which values are identifiers; this renderer only
/// controls their presentation.
#let fact-list(items) = block(width: 100%, below: 0.85em, breakable: true)[
  #for item in items {
    block(width: 100%, breakable: false, above: 0.2em, below: 0.45em)[
      #grid(
        columns: (0.31fr, 0.69fr),
        gutter: 12pt,
        text(
          size: (typ.base_pt - 1) * 1pt,
          weight: "regular",
          fill: primary-dark,
          breakable(item.label),
        ),
        if item.mono {
          mono(breakable(item.value), size: (typ.base_pt - 1) * 1pt)
        } else {
          text(size: (typ.base_pt - 1) * 1pt, breakable(item.value))
        },
      )
    ]
  }
]

// One column grid for the whole metadata section. Group names occupy a full
// row, so nested paths are never squeezed beside a one-word value.
#let metadata-table(groups, keep-together: false) = block(above: 0.25em, below: 0.85em, breakable: not keep-together, table(
  columns: (1fr, 2fr), inset: lay.table_inset_pt * 1pt,
  stroke: table-stroke,
  ..groups.map(group => (
    table.cell(colspan: 2, inset: (x: lay.table_inset_pt * 1pt, y: 5pt),
      text(font: typ.sans, size: typ.table_header_pt * 1pt, weight: "semibold", fill: ink, breakable(group.title))),
    ..group.rows.enumerate().map(((row-index, row)) => (
      table-value(row.first()),
      if group.links.any(entry => entry.row == row-index) {
        external-link(row.last(), row.last(), size: typ.table_pt * 1pt)
      } else { table-value(row.last()) },
    )).flatten(),
  )).flatten(),
))

/// A flowing resource index: the resource name leads, with its Azure context
/// kept on the same visual line where space permits.
#let resource-index(items) = block(width: 100%, below: 0.9em, breakable: true)[
  #for item in items {
    let metadata = (item.subscription, item.resource_group, item.location)
      .filter(value => value != "")
      .join(" · ")
    block(width: 100%, breakable: false, above: 0.18em, below: 0.5em)[
      #text(size: typ.base_pt * 1pt, weight: "regular", fill: if item.target == none { ink } else { accent })[
        #if item.target == none { item.name } else { link(label(item.target), item.name) }
      ]
      #h(8pt)
      #text(size: typ.small_pt * 1pt, fill: muted)[#metadata]
    ]
  }
]

// ----------------------------------------------------------------- stats ----

#let stat(value, label) = {
  let body = [
    #display(cell(value), typ.stat_value_pt * 1pt, weight: "regular", fill: primary-dark) \
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

#let cover-meta(cover) = text(size: typ.small_pt * 1pt, fill: muted)[
  #labels.common.cover.tenant #mono(breakable(cover.tenant)) · #labels.common.cover.snapshot #mono(breakable(cover.snapshot)) \
  #labels.common.cover.collected #cover.collected · #labels.common.cover.status #cover.status
]

// Logos are user-supplied at any aspect ratio, so constrain both axes.
#let cover-logo(logo-path, height: 2.4cm) = if logo-path != "" {
  image(logo-path, width: 6cm, height: height, fit: "contain")
}

#let cover-mark(mark-path) = if mark-path != "" {
  image(mark-path, width: 1.35cm, height: 1.35cm, fit: "contain")
}

#let cover(cover, branding, logo-path, primary-mark-path, on-dark-mark-path) = {
  let mark-path = if lay.cover == "block" { on-dark-mark-path } else { primary-mark-path }
  if lay.cover == "block" {
    page(footer: none, header: none, margin: 0pt, fill: band)[
      #v(1fr)
      #block(inset: (x: 3cm))[
        #if mark-path != "" [#cover-mark(mark-path) #v(0.55cm)]
        #if logo-path != "" [#cover-logo(logo-path) #v(0.8cm)]
        #display(cover.title, typ.title_pt * 1pt, weight: "regular", fill: on-band)
        #if cover.subtitle != "" [
          \ #v(0.2cm) #text(size: typ.subtitle_pt * 1pt, fill: on-band.transparentize(20%))[#cover.subtitle]
        ]
        #if cover.company != "" [
          \ #v(0.4cm) #text(size: typ.subtitle_pt * 1pt, fill: on-band)[#cover.company]
        ]
      ]
      #v(1fr)
      // The muted grey of cover-meta has no contrast on a saturated band, so
      // the block cover recolours it rather than dimming it further.
      #block(inset: (x: 3cm, bottom: 2.5cm), text(fill: on-band)[
        #text(size: typ.small_pt * 1pt)[
          #labels.common.cover.tenant #mono(breakable(cover.tenant)) ·
          #labels.common.cover.snapshot #mono(breakable(cover.snapshot)) \
          #labels.common.cover.collected #cover.collected · #labels.common.cover.status #cover.status
        ]
      ])
    ]
  } else if lay.cover == "band" {
    page(footer: none, header: none, margin: 0pt)[
      #block(width: 100%, height: lay.cover_band_pt * 1pt, fill: band)
      #block(inset: (x: 2.5cm, top: 3cm))[
        #if mark-path != "" [#cover-mark(mark-path) #v(0.55cm)]
        #if logo-path != "" [#cover-logo(logo-path) #v(0.8cm)]
        #display(cover.title, typ.title_pt * 1pt, weight: "regular", fill: primary)
        #if cover.subtitle != "" [
          \ #v(0.2cm) #text(size: typ.subtitle_pt * 1pt, fill: muted)[#cover.subtitle]
        ]
        #if cover.company != "" [
          \ #v(0.4cm) #text(size: typ.subtitle_pt * 1pt, fill: ink)[#cover.company]
        ]
      ]
      #v(1fr)
      #block(inset: (x: 2.5cm, bottom: 2.5cm), cover-meta(cover))
    ]
  } else {
    page(footer: none, header: none)[
      #v(2fr)
      #align(center)[
        #if mark-path != "" [#cover-mark(mark-path) #v(0.65cm)]
        #if logo-path != "" [#cover-logo(logo-path) #v(1cm)]
        #display(cover.title, typ.title_pt * 1pt, fill: ink)
        #v(0.7cm)
        #line(length: 40%, stroke: rule-stroke)
        #v(0.7cm)
        #if cover.subtitle != "" [
          #text(size: typ.subtitle_pt * 1pt, fill: muted)[#cover.subtitle]
          #v(0.35cm)
        ]
        #if cover.company != "" [
          #text(size: typ.subtitle_pt * 1pt, fill: ink)[#cover.company]
        ]
      ]
      #v(1fr)
      #align(center, cover-meta(cover))
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
          text(font: typ.serif, size: typ.h1_pt * 1pt, weight: "regular", fill: primary, it),
        ),
        line(length: 100%, stroke: 1.5pt + primary),
      ),
    )
    #heading(level: 1, title)
  ]
}

/// Name plate above each resource's detail, so a reader scanning a long
/// chapter can find one resource without reading the settings tables.
#let resource-plate(name, subtitle, icon-path) = block(width: 100%, above: 1.4em, below: 0.5em, sticky: true,
  grid(columns: (auto, 1fr), align: horizon, gutter: 6pt,
    if icon-path != "" { image(icon-path, width: typ.h3_pt * 1.2pt, height: typ.h3_pt * 1.2pt, fit: "contain") } else { [] },
    stack(spacing: 3pt,
      text(size: typ.h3_pt * 1pt, weight: "regular", fill: ink, breakable(name)),
      text(size: typ.small_pt * 1pt, fill: muted, subtitle))))

#let sub-label(title) = block(width: 100%, above: 11pt, below: 5pt, sticky: true,
  text(font: typ.sans, size: typ.base_pt * 1pt, weight: "semibold", fill: ink, title))

#let callout(severity, title, detail: none) = block(
  width: 100%,
  stroke: (left: 2.5pt + sev(severity, "text")),
  inset: (x: 9pt, y: 4pt),
  above: 0.35em,
  below: 0.55em,
)[
  #box(
    fill: sev(severity, "fill"),
    radius: 2pt,
    inset: (x: 4pt, y: 2pt),
    text(size: typ.small_pt * 1pt, weight: "bold", fill: sev(severity, "text"), upper(severity)),
  )
  #h(8pt)
  #text(size: typ.base_pt * 1pt)[#title]
  #if detail != none [
    #v(0.18em)
    #text(size: typ.small_pt * 1pt, fill: muted)[#detail]
  ]
]

/// A level-preserving heading with its Azure resource-type icon. Keeping the
/// actual heading element in the cell preserves the outline and running header.
#let icon-heading(level, title, icon-path) = {
  let heading-size = if level == 1 { typ.h1_pt } else { typ.h2_pt }
  [
    #show heading.where(level: level): it => block(
      width: 100%,
      above: 0em,
      below: 0.8em,
      stack(
        spacing: 5pt,
        grid(
          columns: (auto, 1fr),
          align: horizon,
          gutter: 8pt,
          image(
            icon-path,
            width: heading-size * 1pt,
            height: heading-size * 1pt,
            fit: "contain",
          ),
          text(
            size: heading-size * 1pt,
            font: typ.serif,
            weight: "regular",
            fill: primary-dark,
            it,
          ),
        ),

      ),
    )
    #heading(level: level, title)
  ]
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
      counter(page).display(labels.report.pdf.page_counter, both: true),
    ),
  )
}

/// Start a top-level chapter. Themes with divider pages get the heading alone
/// on its own page; the heading itself moves there rather than being repeated
/// after the divider, so the title appears once and the outline still sees it.
#let chapter(title, break_before: false, divider: false) = if lay.divider_pages and divider {
  pagebreak(weak: true)
  v(1fr)
  [
    // A divider page is mostly whitespace, so the title is set larger than an
    // in-flow heading of the same level would be.
    #show heading.where(level: 1): it => block(
      width: 100%,
      stroke: (top: 1.5pt + primary, bottom: 1.5pt + primary),
      inset: (y: 16pt),
      text(font: typ.serif, size: (typ.h1_pt + 10) * 1pt, weight: "regular", fill: primary, it),
    )
    #heading(level: 1, title)
  ]
  v(1.7fr)
  pagebreak()
} else {
  if break_before { pagebreak(weak: true) }
  heading(level: 1, title)
}

/// Document-wide rules. Applied once by report.typ via a show rule.
#let setup(branding, body) = {
  set text(font: typ.sans, size: typ.base_pt * 1pt, fill: ink, lang: labels.meta.lang)
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
    text(font: typ.serif, size: typ.h1_pt * 1pt, weight: "regular", fill: primary, it),
  )
  show heading.where(level: 2): it => block(
    above: 1.5em,
    below: 0.6em,
    text(font: typ.serif, size: typ.h2_pt * 1pt, weight: "regular", fill: primary-dark, it),
  )
  show heading.where(level: 3): it => block(
    above: 1.2em,
    below: 0.5em,
    text(font: typ.serif, size: typ.h3_pt * 1pt, weight: "regular", fill: ink, it),
  )
  show link: set text(fill: accent)
  set figure(numbering: none, gap: 8pt)
  show figure.caption: set align(left)

  body
}
