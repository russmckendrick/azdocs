mod common;

use std::collections::HashSet;

use azdocs::diagram::graph::NamedGraph;
use azdocs::diagram::page::DiagramDetail;
use azdocs::diagram::{DiagramScope, EstateGraph, drawio, mermaid, png, svg};
use azdocs::error::StoreError;
use azdocs::store::Store;
use quick_xml::events::Event;

fn seeded() -> (Store, String) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    (store, id)
}

/// Base64 icon payloads are opaque; collapse them so SVG goldens stay
/// reviewable and survive icon-art changes.
fn svg_insta_settings() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(
        r"data:image/svg\+xml;base64,[A-Za-z0-9+/=]+",
        "data:image/svg+xml;base64,[icon]",
    );
    settings
}

/// Same sheet assembly as `azdocs diagram --type workbook`.
fn workbook_xml(store: &Store, id: &str) -> String {
    let scope = DiagramScope::default();
    let network = EstateGraph::network(store, id, &scope).unwrap();
    let peerings = EstateGraph::peerings(store, id, &scope).unwrap();
    let vnets = EstateGraph::per_vnet(store, id, &scope).unwrap();
    let groups = EstateGraph::per_resource_group(store, id, &scope, DiagramDetail::Full).unwrap();
    let mut sheets: Vec<(&str, &EstateGraph)> =
        vec![("Network Topology", &network), ("VNet Peerings", &peerings)];
    for named in &vnets {
        sheets.push((named.sheet_name.as_str(), &named.graph));
    }
    for named in &groups {
        sheets.push((named.sheet_name.as_str(), &named.graph));
    }
    drawio::render_workbook(&sheets)
}

#[test]
fn mermaid_outputs_match_golden_files() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();

    for (name, graph) in [
        ("hierarchy", EstateGraph::hierarchy(&store, &id).unwrap()),
        (
            "resources",
            EstateGraph::resources(&store, &id, &scope).unwrap(),
        ),
        (
            "network",
            EstateGraph::network(&store, &id, &scope).unwrap(),
        ),
    ] {
        insta::assert_snapshot!(format!("mermaid_{name}"), mermaid::render(&graph));
    }
}

#[test]
fn drawio_outputs_match_golden_files() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();

    for (name, graph) in [
        ("hierarchy", EstateGraph::hierarchy(&store, &id).unwrap()),
        (
            "network",
            EstateGraph::network(&store, &id, &scope).unwrap(),
        ),
    ] {
        insta::assert_snapshot!(format!("drawio_{name}"), drawio::render(&graph));
    }
}

fn assert_unique_resolving_ids(xml: &str) {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut ids = HashSet::new();
    let mut references = Vec::new();
    loop {
        match reader.read_event().expect("well-formed XML") {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"mxCell" => {
                for attribute in e.attributes().map(Result::unwrap) {
                    let value = String::from_utf8_lossy(&attribute.value).into_owned();
                    match attribute.key.as_ref() {
                        b"id" => {
                            assert!(ids.insert(value.clone()), "duplicate id {value}");
                        }
                        b"parent" | b"source" | b"target" => references.push(value),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    for reference in references {
        assert!(ids.contains(&reference), "dangling reference {reference}");
    }
}

#[test]
fn drawio_xml_is_well_formed_with_unique_resolving_ids() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();
    let xml = drawio::render(&EstateGraph::network(&store, &id, &scope).unwrap());

    assert_unique_resolving_ids(&xml);
}

#[test]
fn workbook_xml_has_file_wide_unique_resolving_ids() {
    let (store, id) = seeded();

    assert_unique_resolving_ids(&workbook_xml(&store, &id));
}

#[test]
fn workbook_matches_golden_file() {
    let (store, id) = seeded();

    insta::assert_snapshot!("drawio_workbook", workbook_xml(&store, &id));
}

#[test]
fn svg_outputs_match_golden_files() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();

    svg_insta_settings().bind(|| {
        for (name, graph) in [
            ("hierarchy", EstateGraph::hierarchy(&store, &id).unwrap()),
            (
                "resources",
                EstateGraph::resources(&store, &id, &scope).unwrap(),
            ),
            (
                "network",
                EstateGraph::network(&store, &id, &scope).unwrap(),
            ),
        ] {
            insta::assert_snapshot!(format!("svg_{name}"), svg::render(&graph));
        }
    });
}

#[test]
fn svg_fan_out_graphs_match_golden_files() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();
    let vnets = EstateGraph::per_vnet(&store, &id, &scope).unwrap();
    let groups = EstateGraph::per_resource_group(&store, &id, &scope, DiagramDetail::Full).unwrap();

    svg_insta_settings().bind(|| {
        insta::assert_snapshot!(
            format!("svg_vnet_{}", vnets[0].slug),
            svg::render(&vnets[0].graph)
        );
        insta::assert_snapshot!(
            format!("svg_rg_{}", groups[0].slug),
            svg::render(&groups[0].graph)
        );
    });
}

#[test]
fn fan_out_builders_are_deterministic_with_sorted_slugs() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();

    type FanOut = fn(&Store, &str, &DiagramScope) -> Result<Vec<NamedGraph>, StoreError>;
    let per_group: FanOut =
        |store, id, scope| EstateGraph::per_resource_group(store, id, scope, DiagramDetail::Full);
    for build in [EstateGraph::per_vnet as FanOut, per_group] {
        let first = build(&store, &id, &scope).unwrap();
        let second = build(&store, &id, &scope).unwrap();

        assert!(!first.is_empty(), "fixture estate produces fan-out graphs");
        let slugs: Vec<&str> = first.iter().map(|n| n.slug.as_str()).collect();
        let mut sorted = slugs.clone();
        sorted.sort();
        assert_eq!(slugs, sorted, "slugs are sorted");
        assert_eq!(
            slugs.len(),
            slugs.iter().collect::<HashSet<_>>().len(),
            "slugs are unique"
        );
        assert_eq!(
            first.len(),
            second.len(),
            "same graphs across identical builds"
        );
        for (a, b) in first.iter().zip(&second) {
            assert_eq!(a.slug, b.slug);
            assert_eq!(a.sheet_name, b.sheet_name);
            assert_eq!(svg::render(&a.graph), svg::render(&b.graph));
        }
    }
}

fn viewbox_size(svg_text: &str) -> (f32, f32) {
    let start = svg_text.find("viewBox=\"").expect("viewBox present") + "viewBox=\"".len();
    let end = svg_text[start..].find('"').expect("closing quote") + start;
    let parts: Vec<f32> = svg_text[start..end]
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    (parts[2], parts[3])
}

fn png_ihdr_size(png_bytes: &[u8]) -> (u32, u32) {
    assert!(
        png_bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
        "PNG signature"
    );
    let width = u32::from_be_bytes(png_bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png_bytes[20..24].try_into().unwrap());
    (width, height)
}

#[test]
fn png_renders_every_graph_at_twice_the_svg_size() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();

    let mut graphs = vec![
        EstateGraph::hierarchy(&store, &id).unwrap(),
        EstateGraph::resources(&store, &id, &scope).unwrap(),
        EstateGraph::network(&store, &id, &scope).unwrap(),
        EstateGraph::peerings(&store, &id, &scope).unwrap(),
    ];
    graphs.extend(
        EstateGraph::per_vnet(&store, &id, &scope)
            .unwrap()
            .into_iter()
            .map(|n| n.graph),
    );
    graphs.extend(
        EstateGraph::per_resource_group(&store, &id, &scope, DiagramDetail::Full)
            .unwrap()
            .into_iter()
            .map(|n| n.graph),
    );

    for graph in &graphs {
        let svg_text = svg::render(graph);
        let (svg_width, svg_height) = viewbox_size(&svg_text);
        let png_bytes = png::from_svg(&svg_text, 2.0).unwrap();

        assert!(!png_bytes.is_empty());
        let (width, height) = png_ihdr_size(&png_bytes);
        assert_eq!(width, (svg_width * 2.0) as u32, "{}", graph.title);
        assert_eq!(height, (svg_height * 2.0) as u32, "{}", graph.title);
    }
}

#[test]
fn network_graph_places_vm_in_subnet_not_nic() {
    let (store, id) = seeded();
    let graph = EstateGraph::network(&store, &id, &DiagramScope::default()).unwrap();

    let labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();
    assert!(
        labels.contains(&"vm-app-01") && !labels.contains(&"vm-app-01-nic"),
        "labels: {labels:?}"
    );
}

#[test]
fn network_graph_deduplicates_bidirectional_peering() {
    let (store, id) = seeded();
    let graph = EstateGraph::network(&store, &id, &DiagramScope::default()).unwrap();

    let peerings = graph
        .edges
        .iter()
        .filter(|e| e.label.as_deref() == Some("Connected"))
        .count();
    assert_eq!(peerings, 1);
}

#[test]
fn resources_graph_scopes_to_subscription() {
    let (store, id) = seeded();
    let scope = DiagramScope {
        subscription: Some("sub-dev".to_owned()),
        resource_group: None,
    };

    let graph = EstateGraph::resources(&store, &id, &scope).unwrap();

    let labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();
    assert!(
        labels.contains(&"web-dev") && !labels.contains(&"vm-app-01"),
        "labels: {labels:?}"
    );
}
