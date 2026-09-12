// azdocs PDF report: a generic renderer for the semantic print document.
// Report composition lives in src/report/document.rs; this file only maps
// exhaustive block kinds onto the Typst design primitives in theme.typ.

#import "/theme.typ": *

#let print-doc = json(bytes(sys.inputs.document))
#let branding = json(bytes(sys.inputs.branding))
#let icons = json(bytes(sys.inputs.icons))
#let logo-path = sys.inputs.at("logo", default: "")
#let product-mark-primary = sys.inputs.at("product-mark-primary", default: "")
#let product-mark-on-dark = sys.inputs.at("product-mark-on-dark", default: "")

#set document(
  title: print-doc.cover.title,
  author: if print-doc.cover.company == "" { () } else { (print-doc.cover.company,) },
)

#show: body => setup(branding, body)

#cover(
  print-doc.cover,
  branding,
  logo-path,
  product-mark-primary,
  product-mark-on-dark,
)

#set page(
  header: running-header(branding),
  footer: running-footer(branding),
)

#outline(title: labels.report.toc_title, depth: print-doc.toc_depth)
#pagebreak()

#let render-runs(runs) = {
  for run in runs {
    if run.style == "mono" {
      mono(breakable(run.text))
    } else if run.style == "strong" {
      text(weight: "bold", run.text)
    } else if run.style == "severity" {
      text(
        fill: sev(run.severity, "text"),
        weight: "bold",
        run.text,
      )
    } else {
      run.text
    }
  }
}

#let render-block(item) = {
  if item.kind == "section" {
    if item.break_before { pagebreak(weak: true) }
    [#heading(level: item.level, item.title)#label(item.id)]
  } else if item.kind == "cross_reference" {
    block(below: 0.6em, text(size: typ.small_pt * 1pt, link(label(item.target), item.title)))
  } else if item.kind == "chart" {
    figure(image("/charts/" + item.slug + ".svg", width: 100%), caption: text(size: typ.small_pt * 1pt, item.caption))
  } else if item.kind == "chapter" {
    chapter(
      item.title,
      break_before: item.break_before,
      divider: item.divider,
    )
  } else if item.kind == "heading" {
    let icon-path = if item.icon == none {
      ""
    } else {
      icons.at(item.icon, default: "")
    }
    if icon-path == "" {
      heading(level: item.level, item.title)
    } else {
      icon-heading(item.level, item.title, icon-path)
    }
  } else if item.kind == "paragraph" {
    block(
      above: 0.1em,
      below: if item.style == "muted" { 0.65em } else { 0.85em },
      text(
        size: if item.style == "muted" { typ.small_pt * 1pt } else { typ.base_pt * 1pt },
        fill: if item.style == "muted" { muted } else { ink },
        render-runs(item.runs),
      ),
    )
  } else if item.kind == "bullet_list" {
    text(size: typ.small_pt * 1pt, fill: muted, list(..item.items.map(value => [#value])))
  } else if item.kind == "statistics" {
    block(
      below: 0.9em,
      stat-row(item.items.map(entry => (entry.value, entry.label))),
    )
  } else if item.kind == "table" {
    print-table(item.style, item.columns, item.rows, item.links)
  } else if item.kind == "facts" {
    fact-list(item.items)
  } else if item.kind == "resource_index" {
    resource-index(item.items)
  } else if item.kind == "resource_plate" {
    [#resource-plate(item.name, icons.at(item.icon, default: ""))#label(item.id)]
  } else if item.kind == "sub_label" {
    sub-label(item.title)
  } else if item.kind == "callout" {
    callout(item.severity, item.title, detail: item.detail)
  } else if item.kind == "diagram" {
    let picture = image("/diagrams/" + item.slug + ".svg", width: 100%)
    if item.caption == none {
      picture
    } else {
      figure(
        picture,
        caption: text(size: typ.small_pt * 1pt, item.caption),
      )
    }
  } else if item.kind == "empty_state" {
    empty-state(item.text)
  }
}

#for item in print-doc.blocks {
  render-block(item)
}
