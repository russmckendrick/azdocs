mod common;

use std::collections::HashSet;

use azdocs::diagram::{DiagramScope, EstateGraph, drawio, mermaid};
use azdocs::store::Store;
use quick_xml::events::Event;

fn seeded() -> (Store, String) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    (store, id)
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

#[test]
fn drawio_xml_is_well_formed_with_unique_resolving_ids() {
    let (store, id) = seeded();
    let scope = DiagramScope::default();
    let xml = drawio::render(&EstateGraph::network(&store, &id, &scope).unwrap());

    let mut reader = quick_xml::Reader::from_str(&xml);
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
