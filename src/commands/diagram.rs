use std::path::{Path, PathBuf};

use anyhow::anyhow;

/// Default export root when no `--out` is given.
const DEFAULT_ROOT: &str = "output";

use crate::cli::{DiagramArgs, DiagramFormat, DiagramType};
use crate::diagram::graph::NamedGraph;
use crate::diagram::page::DiagramDetail;
use crate::diagram::{DiagramScope, EstateGraph, drawio, mermaid, png, svg};
use crate::labels::{Labels, fill};
use crate::store::Store;

pub fn run(store: &Store, args: &DiagramArgs, labels: &Labels) -> anyhow::Result<()> {
    run_with_outputs(store, args, labels).map(|_| ())
}

/// Generate diagrams and return every file written by the request.
///
/// The CLI retains its existing progress output; the desktop app consumes the
/// returned paths so fan-out exports can report their complete manifest.
pub fn run_with_outputs(
    store: &Store,
    args: &DiagramArgs,
    labels: &Labels,
) -> anyhow::Result<Vec<PathBuf>> {
    let snapshot_id = store.resolve_snapshot(&args.snapshot)?;
    let words = &labels.diagram;
    let scope = DiagramScope {
        subscription: args.subscription.clone(),
        resource_group: args.resource_group.clone(),
    };
    match args.diagram_type {
        DiagramType::Hierarchy => single(
            args,
            &EstateGraph::hierarchy(store, &snapshot_id, words)?,
            "hierarchy",
            labels,
        ),
        DiagramType::Resources => single(
            args,
            &EstateGraph::resources(store, &snapshot_id, &scope, words)?,
            "resources",
            labels,
        ),
        DiagramType::Network => single(
            args,
            &EstateGraph::network(store, &snapshot_id, &scope, words)?,
            "network",
            labels,
        ),
        DiagramType::Vnets => fan_out(
            args,
            &EstateGraph::per_vnet(store, &snapshot_id, &scope, words)?,
            "vnets",
            labels,
        ),
        DiagramType::ResourceGroups => fan_out(
            args,
            &EstateGraph::per_resource_group(
                store,
                &snapshot_id,
                &scope,
                DiagramDetail::Full,
                words,
            )?,
            "resource-groups",
            labels,
        ),
        DiagramType::Workbook => workbook(store, &snapshot_id, &scope, args, labels),
    }
}

/// Where a diagram lands when the caller names no explicit output path.
///
/// `root` is `output/` for the CLI and the directory the user picked for the
/// desktop's Exports workspace. One function so the two layouts cannot drift —
/// they already had, on where a workbook's rasters go.
///
/// Callers expand set aliases first (see [`expand_formats`]); the fallback
/// extension is deliberately conspicuous so a caller that forgot is obvious in
/// the filename rather than plausibly wrong.
pub fn default_output_path(
    root: &Path,
    diagram_type: DiagramType,
    format: DiagramFormat,
) -> PathBuf {
    match diagram_type {
        // One file per scope, so this names a directory.
        DiagramType::Vnets | DiagramType::ResourceGroups => {
            root.join("diagrams").join(diagram_type.slug())
        }
        // draw.io holds every sheet in one file; a raster cannot, so it fans
        // out into a directory beside the other fan-out types.
        DiagramType::Workbook if format == DiagramFormat::Drawio => {
            root.join("azdocs-workbook.drawio")
        }
        DiagramType::Workbook => root.join("diagrams").join(diagram_type.slug()),
        _ => root.join(format!(
            "azdocs-{}.{}",
            diagram_type.slug(),
            format.extension().unwrap_or("unexpanded")
        )),
    }
}

/// Expand the multi-format aliases into the concrete formats they stand for.
///
/// Public because the desktop has to pre-expand before it can name output
/// files; it previously had its own copy of the rule.
pub fn expand_formats(format: DiagramFormat) -> Vec<DiagramFormat> {
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
    labels: &Labels,
) -> anyhow::Result<(&'static str, Vec<u8>)> {
    let extension = format
        .extension()
        .ok_or_else(|| anyhow!("{format:?} names a set of formats; expand it first"))?;
    let bytes = match format {
        DiagramFormat::Drawio => drawio::render_for(graph, DiagramDetail::Full).into_bytes(),
        DiagramFormat::Mermaid => mermaid::render(graph).into_bytes(),
        DiagramFormat::Svg => {
            svg::render_for(graph, DiagramDetail::Full, &labels.diagram).into_bytes()
        }
        DiagramFormat::Png => png::from_svg(
            &svg::render_for(graph, DiagramDetail::Full, &labels.diagram),
            png::DEFAULT_SCALE,
        )?,
        DiagramFormat::Both | DiagramFormat::All => unreachable!("rejected above"),
    };
    Ok((extension, bytes))
}

fn warn_if_large(name: &str, graph: &EstateGraph, labels: &Labels) {
    if graph.nodes.len() > mermaid::NODE_WARN_THRESHOLD {
        eprintln!(
            "{}",
            fill(
                &labels.cli.diagram.large_warning,
                &[("name", &name), ("count", &graph.nodes.len())]
            )
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

fn single(
    args: &DiagramArgs,
    graph: &EstateGraph,
    type_name: &str,
    labels: &Labels,
) -> anyhow::Result<Vec<PathBuf>> {
    warn_if_large(type_name, graph, labels);
    let formats = expand_formats(args.format);
    let mut outputs = Vec::new();
    for format in &formats {
        let (extension, content) = render_one(graph, *format, labels)?;
        let out = match (&args.out, formats.len()) {
            (Some(path), 1) => path.clone(),
            (Some(path), _) => path.with_extension(extension),
            (None, _) => default_output_path(Path::new(DEFAULT_ROOT), args.diagram_type, *format),
        };
        write_out(&out, &content)?;
        println!(
            "{}",
            fill(
                &labels.cli.diagram.written,
                &[("name", &type_name), ("path", &out.display())]
            )
        );
        outputs.push(out);
    }
    Ok(outputs)
}

fn fan_out(
    args: &DiagramArgs,
    graphs: &[NamedGraph],
    kind_dir: &str,
    labels: &Labels,
) -> anyhow::Result<Vec<PathBuf>> {
    if graphs.is_empty() {
        println!(
            "{}",
            fill(&labels.cli.diagram.nothing_to_write, &[("kind", &kind_dir)])
        );
        return Ok(Vec::new());
    }
    // Fan-out writes one file per graph, so --out names a directory here.
    let dir = args.out.clone().unwrap_or_else(|| {
        default_output_path(Path::new(DEFAULT_ROOT), args.diagram_type, args.format)
    });
    let formats = expand_formats(args.format);
    let mut outputs = Vec::new();
    for named in graphs {
        warn_if_large(&named.sheet_name, &named.graph, labels);
        for format in &formats {
            let (extension, content) = render_one(&named.graph, *format, labels)?;
            let out = dir.join(format!("{}.{extension}", named.slug));
            write_out(&out, &content)?;
            println!(
                "{}",
                fill(
                    &labels.cli.diagram.sheet_written,
                    &[("name", &named.sheet_name), ("path", &out.display())]
                )
            );
            outputs.push(out);
        }
    }
    Ok(outputs)
}

fn workbook(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
    args: &DiagramArgs,
    labels: &Labels,
) -> anyhow::Result<Vec<PathBuf>> {
    let words = &labels.diagram;
    let formats = match args.format {
        // Naming Mermaid (alone or via `both`) is an error; `all` below means
        // "everything applicable", so it skips instead.
        DiagramFormat::Mermaid | DiagramFormat::Both => anyhow::bail!(
            "{}",
            DiagramType::Workbook
                .unsupported_reason(DiagramFormat::Mermaid)
                .unwrap_or_default()
        ),
        DiagramFormat::All => {
            println!("{}", labels.cli.diagram.skipping_mermaid);
            vec![
                DiagramFormat::Drawio,
                DiagramFormat::Svg,
                DiagramFormat::Png,
            ]
        }
        single => vec![single],
    };

    let network = EstateGraph::network(store, snapshot_id, scope, words)?;
    let peerings = EstateGraph::peerings(store, snapshot_id, scope, words)?;
    let vnets = EstateGraph::per_vnet(store, snapshot_id, scope, words)?;
    let groups =
        EstateGraph::per_resource_group(store, snapshot_id, scope, DiagramDetail::Full, words)?;
    // (sheet name, file slug, graph): sheet names may repeat (same RG name in
    // several subscriptions) but the fan-out slugs are already deduplicated,
    // so rasters keep them — prefixed by kind — as their file stems.
    let mut sheets: Vec<(&str, String, &EstateGraph)> = vec![(
        words.workbook.network_topology.as_str(),
        "network-topology".to_owned(),
        &network,
    )];
    if !peerings.nodes.is_empty() {
        sheets.push((
            words.workbook.vnet_peerings.as_str(),
            "vnet-peerings".to_owned(),
            &peerings,
        ));
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
        warn_if_large(name, graph, labels);
    }

    let mut outputs = Vec::new();
    for format in formats {
        match format {
            DiagramFormat::Drawio => {
                // An --out with an extension names the workbook file itself.
                let out = args
                    .out
                    .clone()
                    .filter(|path| path.extension().is_some())
                    .unwrap_or_else(|| {
                        default_output_path(
                            Path::new(DEFAULT_ROOT),
                            DiagramType::Workbook,
                            DiagramFormat::Drawio,
                        )
                    });
                let named_sheets: Vec<(&str, &EstateGraph)> = sheets
                    .iter()
                    .map(|(name, _, graph)| (*name, *graph))
                    .collect();
                write_out(
                    &out,
                    drawio::render_workbook_for(&named_sheets, DiagramDetail::Full).as_bytes(),
                )?;
                println!(
                    "{}",
                    fill(
                        &labels.cli.diagram.workbook_written,
                        &[("count", &sheets.len()), ("path", &out.display())]
                    )
                );
                outputs.push(out);
            }
            DiagramFormat::Svg | DiagramFormat::Png => {
                // Rasters cannot hold multiple sheets, so each sheet becomes
                // its own file in a diagrams directory.
                let dir = args
                    .out
                    .clone()
                    .filter(|path| path.extension().is_none())
                    .unwrap_or_else(|| {
                        default_output_path(Path::new(DEFAULT_ROOT), DiagramType::Workbook, format)
                    });
                for (name, slug, graph) in &sheets {
                    let (extension, content) = render_one(graph, format, labels)?;
                    let out = dir.join(format!("{slug}.{extension}"));
                    write_out(&out, &content)?;
                    println!(
                        "{}",
                        fill(
                            &labels.cli.diagram.sheet_written,
                            &[("name", name), ("path", &out.display())]
                        )
                    );
                    outputs.push(out);
                }
            }
            _ => unreachable!("filtered above"),
        }
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names_a_single_diagram_after_its_type_and_extension() {
        assert_eq!(
            default_output_path(
                Path::new("output"),
                DiagramType::Network,
                DiagramFormat::Svg
            ),
            Path::new("output/azdocs-network.svg")
        );
        // Mermaid writes .mmd, not .mermaid.
        assert_eq!(
            default_output_path(
                Path::new("output"),
                DiagramType::Hierarchy,
                DiagramFormat::Mermaid
            ),
            Path::new("output/azdocs-hierarchy.mmd")
        );
    }

    #[test]
    fn unit_gives_each_fan_out_type_its_own_directory() {
        assert_eq!(
            default_output_path(
                Path::new("output"),
                DiagramType::Vnets,
                DiagramFormat::Drawio
            ),
            Path::new("output/diagrams/vnets")
        );
        assert_eq!(
            default_output_path(
                Path::new("output"),
                DiagramType::ResourceGroups,
                DiagramFormat::Png
            ),
            Path::new("output/diagrams/resource-groups")
        );
    }

    #[test]
    fn unit_splits_the_workbook_by_whether_the_format_holds_sheets() {
        // draw.io keeps every sheet in one file...
        assert_eq!(
            default_output_path(
                Path::new("output"),
                DiagramType::Workbook,
                DiagramFormat::Drawio
            ),
            Path::new("output/azdocs-workbook.drawio")
        );
        // ...a raster cannot, so it fans out beside the other fan-out types.
        assert_eq!(
            default_output_path(
                Path::new("output"),
                DiagramType::Workbook,
                DiagramFormat::Svg
            ),
            Path::new("output/diagrams/workbook")
        );
    }

    #[test]
    fn unit_roots_everything_at_the_directory_it_is_given() {
        // The desktop passes the directory the user picked, not `output/`.
        assert_eq!(
            default_output_path(
                Path::new("/tmp/exports"),
                DiagramType::Network,
                DiagramFormat::Png
            ),
            Path::new("/tmp/exports/azdocs-network.png")
        );
    }
}
