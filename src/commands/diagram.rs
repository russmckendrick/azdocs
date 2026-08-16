use std::path::PathBuf;

use crate::cli::{DiagramArgs, DiagramFormat, DiagramType};
use crate::diagram::graph::NamedGraph;
use crate::diagram::page::DiagramDetail;
use crate::diagram::{DiagramScope, EstateGraph, drawio, mermaid, png, svg};
use crate::store::Store;

pub fn run(store: &Store, args: &DiagramArgs) -> anyhow::Result<()> {
    let snapshot_id = store.resolve_snapshot(&args.snapshot)?;
    let scope = DiagramScope {
        subscription: args.subscription.clone(),
        resource_group: args.resource_group.clone(),
    };
    match args.diagram_type {
        DiagramType::Hierarchy => single(
            args,
            &EstateGraph::hierarchy(store, &snapshot_id)?,
            "hierarchy",
        ),
        DiagramType::Resources => single(
            args,
            &EstateGraph::resources(store, &snapshot_id, &scope)?,
            "resources",
        ),
        DiagramType::Network => single(
            args,
            &EstateGraph::network(store, &snapshot_id, &scope)?,
            "network",
        ),
        DiagramType::Vnets => fan_out(
            args,
            &EstateGraph::per_vnet(store, &snapshot_id, &scope)?,
            "vnets",
        ),
        DiagramType::ResourceGroups => fan_out(
            args,
            &EstateGraph::per_resource_group(store, &snapshot_id, &scope, DiagramDetail::Full)?,
            "resource-groups",
        ),
        DiagramType::Workbook => workbook(store, &snapshot_id, &scope, args),
    }
}

fn expand_formats(format: DiagramFormat) -> Vec<DiagramFormat> {
    match format {
        DiagramFormat::Both => vec![DiagramFormat::Drawio, DiagramFormat::Mermaid],
        DiagramFormat::All => vec![
            DiagramFormat::Drawio,
            DiagramFormat::Mermaid,
            DiagramFormat::Svg,
            DiagramFormat::Png,
        ],
        single => vec![single],
    }
}

fn render_one(
    graph: &EstateGraph,
    format: DiagramFormat,
) -> anyhow::Result<(&'static str, Vec<u8>)> {
    Ok(match format {
        DiagramFormat::Drawio => ("drawio", drawio::render(graph).into_bytes()),
        DiagramFormat::Mermaid => ("mmd", mermaid::render(graph).into_bytes()),
        DiagramFormat::Svg => (
            "svg",
            svg::render_for(graph, DiagramDetail::Full).into_bytes(),
        ),
        DiagramFormat::Png => (
            "png",
            png::from_svg(
                &svg::render_for(graph, DiagramDetail::Full),
                png::DEFAULT_SCALE,
            )?,
        ),
        DiagramFormat::Both | DiagramFormat::All => unreachable!("expanded by expand_formats"),
    })
}

fn warn_if_large(name: &str, graph: &EstateGraph) {
    if graph.nodes.len() > mermaid::NODE_WARN_THRESHOLD {
        eprintln!(
            "Warning: {name}: {} nodes — large diagrams get unreadable; consider --subscription or --resource-group scoping.",
            graph.nodes.len()
        );
    }
}

fn write_out(path: &PathBuf, content: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn single(args: &DiagramArgs, graph: &EstateGraph, type_name: &str) -> anyhow::Result<()> {
    warn_if_large(type_name, graph);
    let formats = expand_formats(args.format);
    for format in &formats {
        let (extension, content) = render_one(graph, *format)?;
        let out = match (&args.out, formats.len()) {
            (Some(path), 1) => path.clone(),
            (Some(path), _) => path.with_extension(extension),
            (None, _) => PathBuf::from("output").join(format!("azdocs-{type_name}.{extension}")),
        };
        write_out(&out, &content)?;
        println!("{type_name} diagram -> {}", out.display());
    }
    Ok(())
}

fn fan_out(args: &DiagramArgs, graphs: &[NamedGraph], kind_dir: &str) -> anyhow::Result<()> {
    if graphs.is_empty() {
        println!("No {kind_dir} diagrams to write for this snapshot.");
        return Ok(());
    }
    // Fan-out writes one file per graph, so --out names a directory here.
    let dir = args
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("output").join("diagrams").join(kind_dir));
    let formats = expand_formats(args.format);
    for named in graphs {
        warn_if_large(&named.sheet_name, &named.graph);
        for format in &formats {
            let (extension, content) = render_one(&named.graph, *format)?;
            let out = dir.join(format!("{}.{extension}", named.slug));
            write_out(&out, &content)?;
            println!("{} -> {}", named.sheet_name, out.display());
        }
    }
    Ok(())
}

fn workbook(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
    args: &DiagramArgs,
) -> anyhow::Result<()> {
    let formats = match args.format {
        DiagramFormat::Mermaid | DiagramFormat::Both => anyhow::bail!(
            "the workbook is a multi-sheet draw.io file and Mermaid has no sheet concept — \
             use --format drawio, or svg/png for per-sheet rasters"
        ),
        DiagramFormat::All => {
            println!("note: skipping Mermaid — the workbook is draw.io-only");
            vec![
                DiagramFormat::Drawio,
                DiagramFormat::Svg,
                DiagramFormat::Png,
            ]
        }
        single => vec![single],
    };

    let network = EstateGraph::network(store, snapshot_id, scope)?;
    let peerings = EstateGraph::peerings(store, snapshot_id, scope)?;
    let vnets = EstateGraph::per_vnet(store, snapshot_id, scope)?;
    let groups = EstateGraph::per_resource_group(store, snapshot_id, scope, DiagramDetail::Full)?;
    // (sheet name, file slug, graph): sheet names may repeat (same RG name in
    // several subscriptions) but the fan-out slugs are already deduplicated,
    // so rasters keep them — prefixed by kind — as their file stems.
    let mut sheets: Vec<(&str, String, &EstateGraph)> =
        vec![("Network Topology", "network-topology".to_owned(), &network)];
    if !peerings.nodes.is_empty() {
        sheets.push(("VNet Peerings", "vnet-peerings".to_owned(), &peerings));
    }
    for named in &vnets {
        sheets.push((
            named.sheet_name.as_str(),
            format!("vnet-{}", named.slug),
            &named.graph,
        ));
    }
    for named in &groups {
        sheets.push((
            named.sheet_name.as_str(),
            format!("resource-group-{}", named.slug),
            &named.graph,
        ));
    }
    for (name, _, graph) in &sheets {
        warn_if_large(name, graph);
    }

    for format in formats {
        match format {
            DiagramFormat::Drawio => {
                // An --out with an extension names the workbook file itself.
                let out = args
                    .out
                    .clone()
                    .filter(|path| path.extension().is_some())
                    .unwrap_or_else(|| PathBuf::from("output").join("azdocs-workbook.drawio"));
                let named_sheets: Vec<(&str, &EstateGraph)> = sheets
                    .iter()
                    .map(|(name, _, graph)| (*name, *graph))
                    .collect();
                write_out(&out, drawio::render_workbook(&named_sheets).as_bytes())?;
                println!("workbook ({} sheets) -> {}", sheets.len(), out.display());
            }
            DiagramFormat::Svg | DiagramFormat::Png => {
                // Rasters cannot hold multiple sheets, so each sheet becomes
                // its own file in a diagrams directory.
                let dir = args
                    .out
                    .clone()
                    .filter(|path| path.extension().is_none())
                    .unwrap_or_else(|| PathBuf::from("output").join("diagrams"));
                for (name, slug, graph) in &sheets {
                    let (extension, content) = render_one(graph, format)?;
                    let out = dir.join(format!("{slug}.{extension}"));
                    write_out(&out, &content)?;
                    println!("{name} -> {}", out.display());
                }
            }
            _ => unreachable!("filtered above"),
        }
    }
    Ok(())
}
