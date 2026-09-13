//! Composes one evidence-led document for the native print renderers.

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::labels::fill;
use crate::labels::{AssessmentLabels, CheckGuidance};
use crate::model::{EdgeKind, Resource, azure_types, azure_values};
use crate::report::analysis::{GroupProfile, Issue, StudyReason};
use crate::report::governance::{HEALTHY_TAG_COVERAGE_PERCENT, percent, tag_keys};

fn para<'a>(blocks: &mut Vec<Block<'a>>, text: impl Into<Cow<'a, str>>) {
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![normal(text)],
    });
}

fn note<'a>(blocks: &mut Vec<Block<'a>>, text: impl Into<Cow<'a, str>>) {
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Muted,
        runs: vec![normal(text)],
    });
}

fn table<'a>(blocks: &mut Vec<Block<'a>>, columns: &[&str], rows: Vec<Vec<String>>) {
    if !rows.is_empty() {
        let weights = table_weights(columns, &rows);
        blocks.push(Block::Table {
            style: TableKind::Data,
            keep_together: false,
            links: Vec::new(),
            columns: columns
                .iter()
                .zip(weights)
                .map(|(label, weight)| TableColumn {
                    label: Cow::Owned((*label).to_owned()),
                    mono: false,
                    weight,
                })
                .collect(),
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(Cow::Owned).collect())
                .collect(),
        });
    }
}

fn section<'a>(
    blocks: &mut Vec<Block<'a>>,
    level: u8,
    title: impl Into<Cow<'a, str>>,
    id: String,
    break_before: bool,
) {
    blocks.push(Block::Section {
        level,
        title: title.into(),
        id,
        break_before,
    });
}

fn link<'a>(blocks: &mut Vec<Block<'a>>, target: String, title: impl Into<Cow<'a, str>>) {
    blocks.push(Block::CrossReference {
        target,
        title: title.into(),
    });
}

fn issue_id(index: usize) -> String {
    format!("issue_{index}")
}

fn guidance<'a>(words: &'a AssessmentLabels, issue: &Issue) -> &'a CheckGuidance {
    words.checks.get(&issue.query_name).unwrap_or_else(|| {
        words
            .checks
            .get("unknown")
            .expect("built-in labels always contain generic check guidance")
    })
}

fn issue_title<'a>(words: &'a AssessmentLabels, issue: &'a Issue) -> &'a str {
    words
        .checks
        .get(&issue.query_name)
        .map_or(issue.query_name.as_str(), |g| g.title.as_str())
}

fn group_name<'a>(group: &'a GroupProfile, labels: &'a Labels) -> &'a str {
    group
        .name
        .as_deref()
        .unwrap_or(&labels.common.subscription_scope)
}

fn families<'a>(
    resources: impl Iterator<Item = &'a Resource>,
    words: &AssessmentLabels,
) -> Vec<(String, usize)> {
    counts(resources.map(|r| {
        let t = r.azure_type.as_str();
        let label = if t.starts_with("microsoft.network/") {
            &words.family_network
        } else if t.starts_with("microsoft.compute/")
            || t.starts_with("microsoft.web/")
            || t.starts_with("microsoft.containerservice/")
            || t.starts_with("microsoft.app/")
            || t.starts_with("microsoft.desktopvirtualization/")
        {
            &words.family_compute
        } else if t.starts_with("microsoft.storage/")
            || t.starts_with("microsoft.sql/")
            || t.starts_with("microsoft.dbfor")
            || t.starts_with("microsoft.documentdb/")
            || t.starts_with("microsoft.cache/")
        {
            &words.family_data
        } else if t.starts_with("microsoft.keyvault/")
            || t.starts_with("microsoft.managedidentity/")
        {
            &words.family_identity
        } else if t.starts_with("microsoft.insights/")
            || t.starts_with("microsoft.operationalinsights/")
            || t.starts_with("microsoft.recoveryservices/")
            || t.starts_with("microsoft.dataprotection/")
        {
            &words.family_monitoring
        } else if t.starts_with("microsoft.logic/")
            || t.starts_with("microsoft.synapse/")
            || t.starts_with("microsoft.datafactory/")
            || t.starts_with("microsoft.cognitiveservices/")
            || t.starts_with("microsoft.search/")
        {
            &words.family_integration
        } else {
            &words.family_other
        };
        label.to_owned()
    }))
}

fn counts(values: impl Iterator<Item = String>) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    counts
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn chart<'a>(
    blocks: &mut Vec<Block<'a>>,
    branding: &BrandingContext,
    slug: &str,
    values: &[(String, usize)],
    total: usize,
    caption: &'a str,
) {
    if values.is_empty() {
        return;
    }
    let pal = &branding.tokens.palette;
    let typography = &branding.tokens.typography;
    let font = if typography.pdf_use_docx_fonts {
        &typography.docx_sans
    } else {
        &typography.sans
    };
    let height = values.len() * 32 + 8;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"680\" height=\"{height}\" viewBox=\"0 0 680 {height}\"><g font-family=\"{}\" font-size=\"15\" fill=\"{}\">",
        xml(font),
        xml(&pal.ink)
    );
    for (index, (label, count)) in values.iter().enumerate() {
        let y = index * 32 + 20;
        let share = percent(*count, total);
        let width = if total == 0 {
            0.0
        } else {
            *count as f64 / total as f64 * 270.0
        };
        svg.push_str(&format!("<text x=\"0\" y=\"{y}\">{}</text><rect x=\"290\" y=\"{}\" width=\"270\" height=\"12\" fill=\"{}\"/><rect x=\"290\" y=\"{}\" width=\"{width:.2}\" height=\"12\" fill=\"{}\"/><text x=\"674\" y=\"{y}\" text-anchor=\"end\">{count} · {share}%</text>", xml(&crate::model::truncate(label, 36)), y-12, xml(&pal.zebra), y-12, xml(&pal.muted)));
    }
    svg.push_str("</g></svg>");
    blocks.push(Block::Chart {
        slug: slug.to_owned(),
        svg,
        caption: Cow::Borrowed(caption),
    });
}

pub(super) fn build<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    diagrams: &'a [DiagramAsset],
    blocks: &mut Vec<Block<'a>>,
) {
    executive(report, branding, blocks);
    composition(report, branding, blocks);
    architecture(report, branding, diagrams, blocks);
    profiles(report, branding, diagrams, blocks);
    issues(report, branding, blocks);
    governance(report, branding, blocks);
    microsoft_evidence(report, branding, blocks);
    actions(report, branding, blocks);
    coverage(report, branding, blocks);
}

fn executive<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    let a = &report.analysis;
    major_chapter(blocks, w.executive.as_str(), false);
    let affected: BTreeSet<_> = a.issues.iter().flat_map(|i| &i.resource_ids).collect();
    para(
        blocks,
        fill(
            &w.executive_intro,
            &[
                ("resources", &report.totals.resources),
                ("subscriptions", &report.totals.subscriptions),
                ("findings", &report.totals.findings),
                ("checks", &a.issues.len()),
                ("affected", &affected.len()),
            ],
        ),
    );
    blocks.push(Block::Statistics {
        items: vec![
            statistic(
                report.totals.resources.to_string(),
                &labels.common.columns.resources,
            ),
            statistic(a.issues.len().to_string(), &w.issue_patterns),
            statistic(affected.len().to_string(), &w.affected),
        ],
    });
    heading(blocks, 2, w.main_observations.as_str(), None);
    if let Some(location) = report.location_counts.first() {
        para(
            blocks,
            fill(
                &w.executive_location,
                &[
                    ("region", &location.display),
                    ("count", &location.count),
                    ("share", &percent(location.count, report.totals.resources)),
                ],
            ),
        );
    }
    if let Some(subscription) = report.subscriptions.iter().max_by_key(|s| s.resource_count) {
        para(
            blocks,
            fill(
                &w.executive_subscription,
                &[
                    ("subscription", &subscription.display_name),
                    ("count", &subscription.resource_count),
                    (
                        "share",
                        &percent(subscription.resource_count, report.totals.resources),
                    ),
                ],
            ),
        );
    }
    para(
        blocks,
        fill(
            &w.executive_tags,
            &[
                ("tagged", &report.tag_coverage.tagged),
                ("untagged", &report.tag_coverage.untagged),
                ("percent", &report.tag_coverage.percent),
            ],
        ),
    );
    heading(blocks, 2, w.review_first.as_str(), None);
    if a.issues.is_empty() {
        para(blocks, w.executive_empty.as_str());
    }
    for (index, issue) in a.issues.iter().enumerate().take(3) {
        let g = guidance(w, issue);
        para(
            blocks,
            fill(
                &w.executive_issue,
                &[
                    ("title", &issue_title(w, issue)),
                    ("occurrences", &issue.occurrences.len()),
                    ("resources", &issue.resource_ids.len()),
                    ("subscriptions", &issue.subscriptions.len()),
                    ("action", &g.action),
                ],
            ),
        );
        link(blocks, issue_id(index), issue_title(w, issue));
    }
    note(blocks, w.reference_available.as_str());
}

fn composition<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    let c = &labels.common.columns;
    major_chapter(blocks, w.composition.as_str(), true);
    para(blocks, w.composition_intro.as_str());
    heading(blocks, 2, w.subscription_comparison.as_str(), None);
    let a = &report.analysis;
    table(
        blocks,
        &[&c.subscription, &c.resources, &w.groups, &w.affected],
        a.subscriptions
            .iter()
            .map(|(id, name)| {
                let resources = a
                    .resources
                    .values()
                    .filter(|r| &r.subscription_id == id)
                    .count();
                let groups = a
                    .groups
                    .iter()
                    .filter(|g| &g.subscription_id == id && !g.resource_ids.is_empty())
                    .count();
                let affected: BTreeSet<_> = a
                    .issues
                    .iter()
                    .filter_map(|i| i.subscriptions.get(id))
                    .flatten()
                    .collect();
                vec![
                    name.clone(),
                    resources.to_string(),
                    groups.to_string(),
                    affected.len().to_string(),
                ]
            })
            .collect(),
    );
    heading(blocks, 2, w.service_mix.as_str(), None);
    chart(
        blocks,
        branding,
        "estate-families",
        &families(a.resources.values(), w),
        a.resources.len(),
        &w.family_caption,
    );
    heading(blocks, 2, w.geography.as_str(), None);
    chart(
        blocks,
        branding,
        "estate-locations",
        &report
            .location_counts
            .iter()
            .map(|l| (l.display.clone(), l.count))
            .collect::<Vec<_>>(),
        report.totals.resources,
        &w.geography_note,
    );
}

fn edge_label(kind: EdgeKind, labels: &Labels) -> &str {
    labels
        .desktop
        .topology
        .edge_kinds
        .get(kind.as_str())
        .map(String::as_str)
        .unwrap_or(kind.as_str())
}

fn architecture<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    diagrams: &'a [DiagramAsset],
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    let a = &report.analysis;
    major_chapter(blocks, w.architecture.as_str(), true);
    para(blocks, w.architecture_intro.as_str());
    para(
        blocks,
        fill(
            &w.architecture_summary,
            &[
                ("connections", &a.relationships.len()),
                (
                    "cross_group",
                    &a.relationships.iter().filter(|r| r.crosses_group()).count(),
                ),
                ("unresolved", &a.unresolved_relationships),
            ],
        ),
    );
    heading(blocks, 2, w.connection_families.as_str(), None);
    table(
        blocks,
        &[&w.relationship, &w.relationships],
        counts(
            a.relationships
                .iter()
                .map(|r| edge_label(r.kind, labels).to_owned()),
        )
        .into_iter()
        .map(|(name, n)| vec![name, n.to_string()])
        .collect(),
    );
    heading(blocks, 2, w.shared_dependencies.as_str(), None);
    para(blocks, w.shared_note.as_str());
    let mut targets: BTreeMap<&str, (BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
    for r in &a.relationships {
        if matches!(r.kind, EdgeKind::AttachedTo | EdgeKind::PeeredWith) {
            continue;
        }
        if let Some(source_group) = &r.source_group {
            let target = targets.entry(&r.target).or_default();
            target.0.insert(&r.source);
            target.1.insert(source_group);
        }
    }
    let mut targets: Vec<_> = targets
        .into_iter()
        .filter(|(_, (_, groups))| groups.len() > 1)
        .collect();
    targets.sort_by(|(ida, (a, _)), (idb, (b, _))| b.len().cmp(&a.len()).then(ida.cmp(idb)));
    if targets.is_empty() {
        note(blocks, w.none_recorded.as_str());
    }
    table(
        blocks,
        &[&w.dependency, &w.consumers, &w.groups],
        targets
            .iter()
            .take(12)
            .map(|(id, (consumers, groups))| {
                vec![
                    a.display_name(id).to_owned(),
                    consumers.len().to_string(),
                    groups.len().to_string(),
                ]
            })
            .collect(),
    );
    if targets.len() > 12 {
        note(
            blocks,
            fill(
                &w.showing_rows,
                &[("shown", &12), ("total", &targets.len())],
            ),
        );
    }
    heading(blocks, 2, w.network_detail.as_str(), None);
    para(blocks, w.network_note.as_str());
    for asset in diagrams
        .iter()
        .filter(|d| d.slug.starts_with("assessment-network"))
    {
        blocks.push(Block::Diagram {
            slug: Cow::Borrowed(&asset.slug),
            caption: Some(Cow::Borrowed(&w.network_caption)),
        });
    }
    let peerings: Vec<_> = a
        .relationships
        .iter()
        .filter(|r| r.kind == EdgeKind::PeeredWith)
        .collect();
    table(
        blocks,
        &[&w.source, &w.target],
        peerings
            .iter()
            .map(|r| {
                vec![
                    a.display_name(&r.source).to_owned(),
                    a.display_name(&r.target).to_owned(),
                ]
            })
            .collect(),
    );
    heading(blocks, 2, w.private_access.as_str(), None);
    table(
        blocks,
        &[&w.source, &w.target],
        a.relationships
            .iter()
            .filter(|r| r.kind == EdgeKind::PrivateEndpointFor)
            .map(|r| {
                vec![
                    a.display_name(&r.source).to_owned(),
                    a.display_name(&r.target).to_owned(),
                ]
            })
            .collect(),
    );
}

fn profiles<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    diagrams: &'a [DiagramAsset],
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    let c = &labels.common.columns;
    let a = &report.analysis;
    major_chapter(blocks, w.profiles.as_str(), true);
    para(blocks, w.profile_intro.as_str());
    for (index, (id, name)) in a.subscriptions.iter().enumerate() {
        section(
            blocks,
            2,
            name.as_str(),
            format!("subscription_{index}"),
            index > 0,
        );
        let resources: Vec<_> = a
            .resources
            .values()
            .filter(|r| &r.subscription_id == id)
            .collect();
        let mut groups: Vec<_> = a
            .groups
            .iter()
            .filter(|g| &g.subscription_id == id && !g.resource_ids.is_empty())
            .collect();
        let tagged = resources
            .iter()
            .filter(|r| tag_keys(r).next().is_some())
            .count();
        let findings: usize = groups
            .iter()
            .map(|g| g.severities.iter().sum::<usize>())
            .sum();
        let affected: BTreeSet<_> = a
            .issues
            .iter()
            .filter_map(|i| i.subscriptions.get(id))
            .flatten()
            .collect();
        let connections = a
            .relationships
            .iter()
            .filter(|r| {
                r.crosses_group()
                    && [&r.source_group, &r.target_group]
                        .into_iter()
                        .flatten()
                        .any(|g| groups.iter().any(|p| &p.key == g))
            })
            .count();
        para(
            blocks,
            fill(
                &w.profile_summary,
                &[
                    ("resources", &resources.len()),
                    ("groups", &groups.len()),
                    ("tagged", &tagged),
                    ("findings", &findings),
                    ("affected", &affected.len()),
                    ("connections", &connections),
                ],
            ),
        );
        if let Some(largest) = groups.iter().max_by_key(|g| g.resource_ids.len()) {
            para(
                blocks,
                fill(
                    &w.profile_concentration,
                    &[
                        ("group", &group_name(largest, labels)),
                        ("resources", &largest.resource_ids.len()),
                        (
                            "share",
                            &percent(largest.resource_ids.len(), resources.len()),
                        ),
                    ],
                ),
            );
        }
        heading(blocks, 3, w.service_mix.as_str(), None);
        chart(
            blocks,
            branding,
            &format!("subscription-families-{index}"),
            &families(resources.iter().copied(), w),
            resources.len(),
            &w.family_caption,
        );
        table(
            blocks,
            &[&w.region, &c.resources],
            counts(resources.iter().map(|r| {
                r.location
                    .as_deref()
                    .map(|l| azure_values::display_location(l).into_owned())
                    .unwrap_or_else(|| w.unknown.clone())
            }))
            .into_iter()
            .map(|(name, n)| vec![name, n.to_string()])
            .collect(),
        );
        groups.sort_by(|a, b| {
            b.resource_ids
                .len()
                .cmp(&a.resource_ids.len())
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.key.cmp(&b.key))
        });
        heading(blocks, 3, w.group_register.as_str(), None);
        table(
            blocks,
            &[
                &c.resource_group,
                &c.resources,
                &w.occurrences,
                &w.relationships,
            ],
            groups
                .iter()
                .take(4)
                .map(|g| {
                    vec![
                        group_name(g, labels).to_owned(),
                        g.resource_ids.len().to_string(),
                        g.severities.iter().sum::<usize>().to_string(),
                        g.cross_group_connections.to_string(),
                    ]
                })
                .collect(),
        );
        if groups.len() > 4 {
            note(
                blocks,
                fill(&w.showing_rows, &[("shown", &4), ("total", &groups.len())]),
            );
        }
        groups.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.key.cmp(&b.key)));
        for (group_index, group) in groups
            .iter()
            .filter(|g| g.study_reason.is_some())
            .enumerate()
        {
            study(
                report,
                branding,
                diagrams,
                group,
                &format!("study_{index}_{group_index}"),
                group_index > 0,
                blocks,
            );
        }
    }
}

fn study<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    diagrams: &'a [DiagramAsset],
    group: &'a GroupProfile,
    anchor: &str,
    break_before: bool,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    let c = &labels.common.columns;
    let a = &report.analysis;
    section(
        blocks,
        3,
        group_name(group, labels),
        anchor.to_owned(),
        break_before,
    );
    let reason = match group.study_reason {
        Some(StudyReason::Findings) => &w.study_findings,
        Some(StudyReason::Connections) => &w.study_connections,
        _ => &w.study_population,
    };
    note(blocks, reason.as_str());
    let resources: Vec<_> = a.group_resources(group).collect();
    let types = counts(
        resources
            .iter()
            .map(|r| azure_types::display_name(&r.azure_type).to_owned()),
    );
    let tagged = resources
        .iter()
        .filter(|r| tag_keys(r).next().is_some())
        .count();
    para(
        blocks,
        fill(
            &w.study_summary,
            &[
                ("resources", &resources.len()),
                ("types", &types.len()),
                ("findings", &group.severities.iter().sum::<usize>()),
                ("connections", &group.cross_group_connections),
                ("tagged", &tagged),
            ],
        ),
    );
    if let Some((name, count)) = types.first() {
        para(
            blocks,
            fill(
                &w.study_mix,
                &[
                    ("type", name),
                    ("count", count),
                    ("total", &resources.len()),
                ],
            ),
        );
    }
    table(
        blocks,
        &[&c.r#type, &c.resources],
        types
            .iter()
            .take(6)
            .map(|(name, n)| vec![name.clone(), n.to_string()])
            .collect(),
    );
    if types.len() > 6 {
        note(
            blocks,
            fill(&w.showing_rows, &[("shown", &6), ("total", &types.len())]),
        );
    }
    for asset in diagrams
        .iter()
        .filter(|d| d.group_key.as_deref() == Some(&group.key) && d.slug.starts_with("assessment-"))
    {
        blocks.push(Block::Diagram {
            slug: Cow::Borrowed(&asset.slug),
            caption: Some(Cow::Owned(format!(
                "{} {}",
                asset.title,
                fill(&w.diagram_caption, &[("group", &group_name(group, labels))])
            ))),
        });
    }
    heading(blocks, 3, w.configuration.as_str(), None);
    para(blocks, w.configuration_note.as_str());
    configuration_tables(report, &resources, labels, blocks);
    heading(blocks, 3, w.group_dependencies.as_str(), None);
    let relationships: Vec<_> = a
        .relationships
        .iter()
        .filter(|r| {
            r.crosses_group()
                && (r.source_group.as_deref() == Some(&group.key)
                    || r.target_group.as_deref() == Some(&group.key))
        })
        .collect();
    if relationships.is_empty() {
        para(blocks, w.group_dependencies_none.as_str());
    } else {
        para(
            blocks,
            fill(
                &w.study_dependency_summary,
                &[
                    (
                        "incoming",
                        &relationships
                            .iter()
                            .filter(|r| {
                                r.kind != EdgeKind::PeeredWith
                                    && r.target_group.as_deref() == Some(&group.key)
                            })
                            .count(),
                    ),
                    (
                        "outgoing",
                        &relationships
                            .iter()
                            .filter(|r| {
                                r.kind != EdgeKind::PeeredWith
                                    && r.source_group.as_deref() == Some(&group.key)
                            })
                            .count(),
                    ),
                    (
                        "peerings",
                        &relationships
                            .iter()
                            .filter(|r| r.kind == EdgeKind::PeeredWith)
                            .count(),
                    ),
                ],
            ),
        );
    }
    table(
        blocks,
        &[&w.source, &w.relationship, &w.target],
        relationships
            .iter()
            .take(12)
            .map(|r| {
                vec![
                    a.display_name(&r.source).to_owned(),
                    edge_label(r.kind, labels).to_owned(),
                    a.display_name(&r.target).to_owned(),
                ]
            })
            .collect(),
    );
    if relationships.len() > 12 {
        note(
            blocks,
            fill(
                &w.showing_rows,
                &[("shown", &12), ("total", &relationships.len())],
            ),
        );
    }
    heading(blocks, 3, w.study_findings_heading.as_str(), None);
    let mut observations = Vec::new();
    let mut observation_links = Vec::new();
    for (index, issue) in a.issues.iter().enumerate() {
        if let Some(ids) = issue.groups.get(&group.key) {
            observation_links.push(TableLink {
                row: observations.len(),
                column: 0,
                target: issue_id(index),
                external: false,
            });
            observations.push(vec![
                issue_title(w, issue).to_owned(),
                ids.len().to_string(),
            ]);
        }
    }
    if observations.is_empty() {
        para(blocks, w.study_findings_none.as_str());
    } else {
        table(blocks, &[&c.check, &w.affected], observations);
        if let Some(Block::Table { links, .. }) = blocks.last_mut() {
            *links = observation_links;
        }
    }
}

fn value(
    resource: &Resource,
    pointer: &str,
    report: &ReportContext,
    row_key: &str,
    words: &AssessmentLabels,
) -> String {
    let direct = if pointer.starts_with("/sku") {
        resource
            .sku
            .as_ref()
            .and_then(|v| v.pointer(pointer.trim_start_matches("/sku")))
    } else if pointer.starts_with("/identity") {
        resource
            .identity
            .as_ref()
            .and_then(|v| v.pointer(pointer.trim_start_matches("/identity")))
    } else {
        resource.properties.as_ref().and_then(|v| {
            v.pointer(pointer).or_else(|| {
                (pointer == "/publicNetworkAccess")
                    .then(|| v.pointer("/network/publicNetworkAccess"))
                    .flatten()
            })
        })
    };
    let recorded = direct.filter(|v| !v.is_null()).or_else(|| {
        report
            .analysis
            .recorded_queries
            .values()
            .flatten()
            .find_map(|row| {
                (row.get("id")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|id| id.eq_ignore_ascii_case(&resource.id)))
                .then(|| row.get(row_key))
                .flatten()
                .filter(|v| !v.is_null())
            })
    });
    recorded
        .map(|v| cell_to_string(Some(v)))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| words.unknown.clone())
}

fn configuration_tables<'a>(
    report: &ReportContext,
    resources: &[&Resource],
    labels: &'a Labels,
    blocks: &mut Vec<Block<'a>>,
) {
    let w = &labels.report.assessment;
    let c = &labels.common.columns;
    let mut any = false;
    for (family, title, headers, fields) in [
        (
            "compute",
            &w.compute,
            vec![&w.vm_size, &w.operating_system, &w.zones],
            vec![
                ("/hardwareProfile/vmSize", "vmSize"),
                ("/storageProfile/osDisk/osType", "osType"),
                ("/zones", "zones"),
            ],
        ),
        (
            "storage",
            &w.storage,
            vec![&w.replication, &w.public_network, &w.blob_access],
            vec![
                ("/sku/name", "sku"),
                ("/publicNetworkAccess", "publicNetworkAccess"),
                ("/allowBlobPublicAccess", "allowBlobPublicAccess"),
            ],
        ),
        (
            "database",
            &w.databases,
            vec![&w.public_network, &w.default_action],
            vec![
                ("/publicNetworkAccess", "publicNetworkAccess"),
                ("/networkAcls/defaultAction", "defaultNetworkAction"),
            ],
        ),
    ] {
        let members: Vec<_> = resources
            .iter()
            .filter(|r| match family {
                "compute" => r.azure_type == "microsoft.compute/virtualmachines",
                "storage" => r.azure_type == "microsoft.storage/storageaccounts",
                _ => matches!(
                    r.azure_type.as_str(),
                    "microsoft.sql/servers"
                        | "microsoft.documentdb/databaseaccounts"
                        | "microsoft.dbforpostgresql/flexibleservers"
                        | "microsoft.dbformysql/flexibleservers"
                        | "microsoft.dbforpostgresql/servers"
                        | "microsoft.dbformysql/servers"
                ),
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        any = true;
        sub_label(blocks, title);
        let columns: Vec<_> = std::iter::once(c.resource.as_str())
            .chain(headers.iter().map(|s| s.as_str()))
            .collect();
        table(
            blocks,
            &columns,
            members
                .iter()
                .take(12)
                .map(|r| {
                    std::iter::once(r.name.clone())
                        .chain(
                            fields
                                .iter()
                                .map(|(pointer, key)| value(r, pointer, report, key, w)),
                        )
                        .collect()
                })
                .collect(),
        );
        if members.len() > 12 {
            note(
                blocks,
                fill(
                    &w.showing_rows,
                    &[("shown", &12), ("total", &members.len())],
                ),
            );
        }
    }
    let identity: Vec<_> = resources
        .iter()
        .filter(|r| {
            r.identity
                .as_ref()
                .and_then(|v| v.get("type"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|v| v != "None")
                || report.analysis.relationships.iter().any(|edge| {
                    edge.source == r.id
                        && matches!(
                            edge.kind,
                            EdgeKind::LogsTo | EdgeKind::UsesIdentity | EdgeKind::RunsOn
                        )
                })
        })
        .collect();
    if !identity.is_empty() {
        any = true;
        sub_label(blocks, &w.identity_logging);
        table(
            blocks,
            &[&c.resource, &w.identity_type, &w.dependency],
            identity
                .iter()
                .take(10)
                .map(|r| {
                    let related: Vec<_> = report
                        .analysis
                        .relationships
                        .iter()
                        .filter(|edge| {
                            edge.source == r.id
                                && matches!(
                                    edge.kind,
                                    EdgeKind::LogsTo | EdgeKind::UsesIdentity | EdgeKind::RunsOn
                                )
                        })
                        .map(|edge| {
                            format!(
                                "{}: {}",
                                edge_label(edge.kind, labels),
                                report.analysis.display_name(&edge.target)
                            )
                        })
                        .collect();
                    vec![
                        r.name.clone(),
                        value(r, "/identity/type", report, "identityType", w),
                        if related.is_empty() {
                            w.none_recorded.clone()
                        } else {
                            related.join("\n")
                        },
                    ]
                })
                .collect(),
        );
        if identity.len() > 10 {
            note(
                blocks,
                fill(
                    &w.showing_rows,
                    &[("shown", &10), ("total", &identity.len())],
                ),
            );
        }
    }
    if !any {
        para(blocks, w.configuration_absent.as_str());
    }
}

fn issues<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    major_chapter(blocks, w.security.as_str(), true);
    para(blocks, w.security_intro.as_str());
    if report.analysis.issues.is_empty() {
        para(blocks, w.executive_empty.as_str());
    }
    for (index, issue) in report.analysis.issues.iter().enumerate() {
        let g = guidance(w, issue);
        section(blocks, 2, issue_title(w, issue), issue_id(index), index > 0);
        blocks.push(Block::Callout {
            severity: Cow::Borrowed(issue.severity()),
            title: Cow::Owned(fill(
                &w.issue_summary,
                &[
                    ("occurrences", &issue.occurrences.len()),
                    ("resources", &issue.resource_ids.len()),
                    ("estate_level", &issue.estate_level),
                    ("unresolved", &issue.unresolved),
                ],
            )),
            detail: None,
        });
        table(
            blocks,
            &[&w.severity_mix, &w.occurrences],
            ["high", "medium", "low", "info"]
                .into_iter()
                .zip(issue.severities)
                .filter(|(_, n)| *n > 0)
                .map(|(key, n)| vec![labels.common.severity.get(key).label.clone(), n.to_string()])
                .collect(),
        );
        for (title, text) in [
            (&w.issue_observation, &g.observation),
            (&w.issue_significance, &g.significance),
            (&w.issue_verification, &g.verification),
            (&w.issue_policy, &g.policy),
        ] {
            sub_label(blocks, title);
            para(blocks, text.as_str());
        }
        sub_label(blocks, &w.issue_scope);
        table(
            blocks,
            &[&labels.common.columns.subscription, &w.affected],
            issue
                .subscriptions
                .iter()
                .map(|(id, ids)| {
                    vec![
                        report.analysis.subscriptions.get(id).unwrap_or(id).clone(),
                        ids.len().to_string(),
                    ]
                })
                .collect(),
        );
        if let Some((key, ids)) = issue
            .groups
            .iter()
            .max_by(|(ka, a), (kb, b)| a.len().cmp(&b.len()).then(kb.cmp(ka)))
            && let Some(group) = report.analysis.groups.iter().find(|g| &g.key == key)
        {
            para(
                blocks,
                fill(
                    &w.issue_group_scope,
                    &[
                        ("count", &ids.len()),
                        ("group", &group_name(group, labels)),
                        (
                            "subscription",
                            &report
                                .analysis
                                .subscriptions
                                .get(&group.subscription_id)
                                .unwrap_or(&group.subscription_id),
                        ),
                    ],
                ),
            );
        }
        sub_label(blocks, &w.issue_examples);
        blocks.push(Block::BulletList {
            items: issue
                .occurrences
                .iter()
                .take(3)
                .map(|occurrence| Cow::Borrowed(occurrence.title.as_str()))
                .collect(),
        });
        note(
            blocks,
            fill(
                &w.example_limit,
                &[
                    ("shown", &issue.occurrences.len().min(3)),
                    ("total", &issue.occurrences.len()),
                ],
            ),
        );
        if !g.source.is_empty() {
            if g.source.starts_with("https://") || g.source.starts_with("http://") {
                blocks.push(Block::ExternalLink {
                    title: Cow::Borrowed(&w.source),
                    url: Cow::Borrowed(&g.source),
                });
            } else {
                note(blocks, g.source.as_str());
            }
        }
    }
}

fn governance<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    let c = &labels.common.columns;
    major_chapter(blocks, w.governance.as_str(), true);
    para(blocks, w.governance_intro.as_str());
    para(
        blocks,
        fill(
            &w.governance_coverage,
            &[
                ("tagged", &report.tag_coverage.tagged),
                ("total", &report.totals.resources),
                ("percent", &report.tag_coverage.percent),
                ("threshold", &HEALTHY_TAG_COVERAGE_PERCENT),
            ],
        ),
    );
    if let Some(lowest) = report
        .governance
        .subscriptions
        .iter()
        .min_by_key(|s| s.percent)
    {
        para(
            blocks,
            fill(
                &w.tag_focus,
                &[
                    ("subscription", &lowest.display_name),
                    ("percent", &lowest.percent),
                ],
            ),
        );
    }
    heading(
        blocks,
        2,
        labels.common.governance.coverage_by_subscription.as_str(),
        None,
    );
    table(
        blocks,
        &[&c.subscription, &c.tag_coverage],
        report
            .governance
            .subscriptions
            .iter()
            .map(|s| vec![s.display_name.clone(), format!("{}%", s.percent)])
            .collect(),
    );
    heading(
        blocks,
        2,
        labels.common.governance.coverage_by_key.as_str(),
        None,
    );
    note(
        blocks,
        fill(
            &labels.common.governance.key_share,
            &[("tagged", &report.tag_coverage.tagged)],
        ),
    );
    table(
        blocks,
        &[&c.tag_key, &c.resources, &c.share],
        report
            .governance
            .top_keys
            .iter()
            .map(|k| {
                vec![
                    k.key.clone(),
                    k.count.to_string(),
                    format!("{}%", k.percent),
                ]
            })
            .collect(),
    );
    if report.governance.non_compliant == 0 {
        para(blocks, w.tag_scope_unknown.as_str());
    } else {
        para(
            blocks,
            fill(
                &w.tag_scope_known,
                &[("count", &report.governance.non_compliant)],
            ),
        );
        table(
            blocks,
            &[&c.resource_group, &c.non_compliant, &c.missed_tags],
            report
                .governance
                .worst_groups
                .iter()
                .map(|g| {
                    vec![
                        format!("{} / {}", g.subscription_name, g.name),
                        g.non_compliant.to_string(),
                        g.missed_tags.join(", "),
                    ]
                })
                .collect(),
        );
    }
    heading(blocks, 2, w.operations.as_str(), None);
    let a = &report.analysis;
    let logging: BTreeSet<_> = a
        .relationships
        .iter()
        .filter(|r| r.kind == EdgeKind::LogsTo)
        .map(|r| &r.target)
        .collect();
    let sources: BTreeSet<_> = a
        .relationships
        .iter()
        .filter(|r| r.kind == EdgeKind::LogsTo)
        .map(|r| &r.source)
        .collect();
    let vaults: Vec<_> = a
        .resources
        .values()
        .filter(|r| {
            matches!(
                r.azure_type.as_str(),
                "microsoft.recoveryservices/vaults" | "microsoft.dataprotection/backupvaults"
            )
        })
        .collect();
    para(
        blocks,
        fill(
            &w.operations_summary,
            &[
                ("logging", &logging.len()),
                ("sources", &sources.len()),
                (
                    "monitors",
                    &a.relationships
                        .iter()
                        .filter(|r| r.kind == EdgeKind::Monitors)
                        .count(),
                ),
                ("vaults", &vaults.len()),
            ],
        ),
    );
    para(blocks, w.operations_note.as_str());
    table(
        blocks,
        &[&w.logging_targets, &w.consumers],
        logging
            .iter()
            .map(|id| {
                vec![
                    a.display_name(id).to_owned(),
                    a.relationships
                        .iter()
                        .filter(|r| r.kind == EdgeKind::LogsTo && &r.target == *id)
                        .map(|r| &r.source)
                        .collect::<BTreeSet<_>>()
                        .len()
                        .to_string(),
                ]
            })
            .collect(),
    );
    table(
        blocks,
        &[&w.vault_configuration, &c.location],
        vaults
            .iter()
            .map(|r| {
                vec![
                    r.name.clone(),
                    r.location
                        .as_deref()
                        .map(|l| azure_values::display_location(l).into_owned())
                        .unwrap_or_else(|| w.unknown.clone()),
                ]
            })
            .collect(),
    );
    section(
        blocks,
        2,
        w.operational_questions.as_str(),
        "operational-review".to_owned(),
        false,
    );
    para(blocks, w.operational_review.as_str());
}

fn actions<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    major_chapter(blocks, w.actions.as_str(), true);
    para(blocks, w.actions_intro.as_str());
    table(
        blocks,
        &[&w.action, &labels.common.columns.severity, &w.affected],
        report
            .analysis
            .issues
            .iter()
            .map(|issue| {
                vec![
                    guidance(w, issue).action.clone(),
                    labels.common.severity.get(issue.severity()).label.clone(),
                    issue.resource_ids.len().to_string(),
                ]
            })
            .collect(),
    );
    if !report.analysis.issues.is_empty()
        && let Some(Block::Table { links, .. }) = blocks.last_mut()
    {
        *links = report
            .analysis
            .issues
            .iter()
            .enumerate()
            .map(|(row, _)| TableLink {
                row,
                column: 0,
                target: issue_id(row),
                external: false,
            })
            .collect();
    }
    link(
        blocks,
        "operational-review".to_owned(),
        w.operational_questions.as_str(),
    );
}

fn coverage<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    major_chapter(blocks, w.coverage.as_str(), true);
    para(blocks, w.coverage_intro.as_str());
    let runs = &report.analysis.query_runs;
    let failed = runs.iter().filter(|r| r.error.is_some()).count();
    let successful = runs
        .iter()
        .filter(|r| r.error.is_none() && r.row_count.is_some())
        .count();
    let empty = runs
        .iter()
        .filter(|r| r.error.is_none() && r.row_count == Some(0))
        .count();
    para(
        blocks,
        fill(
            &w.coverage_summary,
            &[
                ("status", &report.status),
                ("successful", &successful),
                ("empty", &empty),
                ("failed", &failed),
                ("unknown", &(runs.len() - failed - successful)),
            ],
        ),
    );
    blocks.push(Block::Facts {
        items: vec![
            fact(&labels.common.cover.snapshot, &report.snapshot_id, true),
            fact(&labels.common.cover.collected, &report.created_at, false),
            fact(&labels.common.cover.tenant, &report.tenant_id, true),
        ],
    });
    if runs.is_empty() {
        super::empty(blocks, &w.coverage_missing);
    }
    table(
        blocks,
        &[&w.query, &w.result, &w.rows],
        runs.iter()
            .map(|r| {
                let outcome = if r.error.is_some() {
                    &w.failed
                } else if r.row_count == Some(0) {
                    &w.empty_query
                } else if r.row_count.is_some() {
                    &w.successful
                } else {
                    &w.outcome_unknown
                };
                vec![
                    display_label(&r.query_name),
                    outcome.clone(),
                    r.row_count
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| w.unknown.clone()),
                ]
            })
            .collect(),
    );
    for run in runs.iter().filter(|run| run.error.is_some()) {
        heading(blocks, 3, display_label(&run.query_name), None);
        if let Some(error) = &run.error {
            para(blocks, error.as_str());
        }
    }
    para(blocks, w.coverage_limits.as_str());
    note(blocks, w.guidance_scope.as_str());
}

pub(super) fn reference<'a>(
    report: &'a ReportContext,
    branding: &'a BrandingContext,
    diagrams: &'a [DiagramAsset],
    blocks: &mut Vec<Block<'a>>,
) {
    let labels = &branding.labels;
    let w = &labels.report.assessment;
    major_chapter(blocks, w.reference_title.as_str(), false);
    para(blocks, w.reference_intro.as_str());
    para(blocks, w.reference_reductions.as_str());
    build_type_index(report, labels, blocks);
    for (id, name) in &report.analysis.subscriptions {
        chapter(blocks, name.as_str(), true);
        for group in report
            .analysis
            .groups
            .iter()
            .filter(|g| &g.subscription_id == id && !g.resource_ids.is_empty())
        {
            heading(blocks, 2, group_name(group, labels), None);
            for asset in diagrams.iter().filter(|d| {
                d.group_key.as_deref() == Some(&group.key) && !d.slug.starts_with("assessment-")
            }) {
                blocks.push(Block::Diagram {
                    slug: Cow::Borrowed(&asset.slug),
                    caption: Some(Cow::Borrowed(group_name(group, labels))),
                });
            }
            for resource in report.analysis.group_resources(group) {
                blocks.push(Block::ResourcePlate {
                    id: resource_anchor(report, &resource.id).unwrap_or_default(),
                    name: Cow::Borrowed(&resource.name),
                    icon: Cow::Borrowed(&resource.azure_type),
                    subtitle: Cow::Borrowed(azure_types::display_name(&resource.azure_type)),
                });
                super::metadata::identity(resource, name, labels, blocks);
                report.websites.print_blocks(&resource.id, labels, blocks);
                super::metadata::configuration(resource, labels, blocks);
                let related: Vec<_> = report
                    .analysis
                    .relationships
                    .iter()
                    .filter(|r| r.source == resource.id || r.target == resource.id)
                    .collect();
                if !related.is_empty() {
                    sub_label(blocks, &w.reference_relationships);
                }
                table(
                    blocks,
                    &[&w.source, &w.relationship, &w.target],
                    related
                        .iter()
                        .map(|r| {
                            vec![
                                report.analysis.display_name(&r.source).to_owned(),
                                edge_label(r.kind, labels).to_owned(),
                                report.analysis.display_name(&r.target).to_owned(),
                            ]
                        })
                        .collect(),
                );
                if !related.is_empty()
                    && let Some(Block::Table { links, .. }) = blocks.last_mut()
                {
                    for (row, relation) in related.iter().enumerate() {
                        for (column, id) in [(0, &relation.source), (2, &relation.target)] {
                            if let Some(target) = resource_anchor(report, id) {
                                links.push(TableLink {
                                    row,
                                    column,
                                    target,
                                    external: false,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    major_chapter(blocks, w.reference_occurrences.as_str(), true);
    for issue in &report.analysis.issues {
        heading(blocks, 2, issue_title(w, issue), None);
        for occurrence in &issue.occurrences {
            note(
                blocks,
                format!(
                    "{} · {}",
                    labels
                        .common
                        .severity
                        .get(occurrence.severity.as_str())
                        .label,
                    occurrence.query_name
                ),
            );
            para(blocks, occurrence.title.as_str());
            if let Some(id) = &occurrence.resource_id {
                if let Some(target) = resource_anchor(report, id) {
                    link(blocks, target, id.as_str());
                } else {
                    note(blocks, id.as_str());
                }
            }
            if let Some(evidence) = &occurrence.detail {
                json_facts(blocks, &w.original_evidence, evidence);
            }
        }
    }
    major_chapter(blocks, labels.report.evidence.chapter.as_str(), true);
    para(blocks, w.reference_reductions.as_str());
    for (name, rows) in &report.analysis.recorded_queries {
        heading(blocks, 2, display_label(name), None);
        let columns = crate::model::rows::columns(rows);
        let selected = super::super::page_columns(&columns);
        note(
            blocks,
            fill(
                &w.reference_query_note,
                &[("shown", &selected.len()), ("total", &columns.len())],
            ),
        );
        table(
            blocks,
            &selected,
            rows.iter()
                .map(|row| {
                    selected
                        .iter()
                        .map(|key| print_evidence_value(row.get(*key)))
                        .collect()
                })
                .collect(),
        );
    }
}

fn json_facts<'a>(blocks: &mut Vec<Block<'a>>, prefix: &str, value: &serde_json::Value) {
    fn flatten(prefix: &str, value: &serde_json::Value, items: &mut Vec<Fact<'static>>) {
        match value {
            serde_json::Value::Object(map) if !map.is_empty() => {
                for (key, value) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    flatten(&path, value, items);
                }
            }
            _ => items.push(fact(
                prefix.to_owned(),
                print_evidence_value(Some(value)),
                false,
            )),
        }
    }
    let mut items = Vec::new();
    flatten(prefix, value, &mut items);
    blocks.push(Block::Facts { items });
}

// Long URLs remain complete even inside stored JSON/query evidence.
fn print_evidence_value(value: Option<&serde_json::Value>) -> String {
    let text = cell_to_string(value);
    if text.contains("https://") || text.contains("http://") {
        text
    } else {
        crate::model::truncate(&text, 512)
    }
}

fn microsoft_evidence<'a>(
    report: &ReportContext,
    branding: &'a BrandingContext,
    blocks: &mut Vec<Block<'a>>,
) {
    let w = &branding.labels.report.posture;
    major_chapter(blocks, w.chapter.as_str(), true);
    para(blocks, w.intro.as_str());
    for evidence in report.posture.tables(&branding.labels) {
        heading(blocks, 2, evidence.title, None);
        let total = evidence.rows.len();
        let headers: Vec<&str> = evidence.columns.iter().map(String::as_str).collect();
        table(
            blocks,
            &headers,
            evidence.rows.into_iter().take(20).collect(),
        );
        para(blocks, evidence.status);
        note(blocks, evidence.note);
        if total > 20 {
            note(
                blocks,
                fill(
                    &branding.labels.report.assessment.showing_rows,
                    &[("shown", &20), ("total", &total)],
                ),
            );
        }
    }
}
