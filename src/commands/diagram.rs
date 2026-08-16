use std::path::PathBuf;

use crate::cli::{DiagramArgs, DiagramFormat, DiagramType};
use crate::diagram::{DiagramScope, EstateGraph, drawio, mermaid};
use crate::store::Store;

pub fn run(store: &Store, args: &DiagramArgs) -> anyhow::Result<()> {
    let snapshot_id = store.resolve_snapshot(&args.snapshot)?;
    let scope = DiagramScope {
        subscription: args.subscription.clone(),
        resource_group: args.resource_group.clone(),
    };
    let (graph, type_name) = match args.diagram_type {
        DiagramType::Hierarchy => (EstateGraph::hierarchy(store, &snapshot_id)?, "hierarchy"),
        DiagramType::Resources => (
            EstateGraph::resources(store, &snapshot_id, &scope)?,
            "resources",
        ),
        DiagramType::Network => (
            EstateGraph::network(store, &snapshot_id, &scope)?,
            "network",
        ),
    };

    if graph.nodes.len() > mermaid::NODE_WARN_THRESHOLD {
        eprintln!(
            "Warning: {} nodes — large diagrams get unreadable; consider --subscription or --resource-group scoping.",
            graph.nodes.len()
        );
    }

    let formats: &[DiagramFormat] = match args.format {
        DiagramFormat::Both => &[DiagramFormat::Drawio, DiagramFormat::Mermaid],
        single => &[single],
    };
    for format in formats {
        let (extension, content) = match format {
            DiagramFormat::Drawio => ("drawio", drawio::render(&graph)),
            DiagramFormat::Mermaid => ("mmd", mermaid::render(&graph)),
            DiagramFormat::Both => unreachable!("expanded above"),
        };
        let out = match (&args.out, formats.len()) {
            (Some(path), 1) => path.clone(),
            (Some(path), _) => path.with_extension(extension),
            (None, _) => PathBuf::from(format!("azdocs-{type_name}.{extension}")),
        };
        std::fs::write(&out, content)?;
        println!("{type_name} diagram -> {}", out.display());
    }
    Ok(())
}
