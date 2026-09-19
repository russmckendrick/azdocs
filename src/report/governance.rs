//! Tag governance: the thresholds, and the analysis that applies them.
//!
//! Both the printed report and the desktop explorer answer the same three
//! questions — how much of the estate is tagged, which keys and subscriptions
//! carry that coverage, and which resource groups are worst at the required
//! tags. The explorer used to answer them itself in a `useMemo`, which is why
//! the wire contract carried threshold *values* for the frontend to compare
//! against and the report could not show the table at all. The analysis lives
//! here so there is one implementation and both surfaces read its verdicts.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::model::{Finding, Resource, ResourceGroup, Subscription};

/// The audit that records a resource missing a required tag.
///
/// The analysis reads the stored findings rather than re-comparing tags
/// against today's config, so a report and the explorer describe the estate as
/// it was audited — editing `required_tags` after a collection no longer makes
/// the two disagree about the same snapshot.
pub(crate) const TAG_AUDIT: &str = "missing_required_tags";

/// Shown where a resource has no resource group of its own.
const UNGROUPED: &str = "—";

/// How many rows the summaries keep. Past this a summary stops summarising;
/// the full picture is in the findings and the evidence appendix.
const TOP_TAG_KEYS: usize = 6;
const WORST_GROUPS: usize = 6;

/// Tag coverage at or above this reads as healthy.
///
/// A threshold is a product judgement, not a rendering detail, so it lives
/// here rather than in whichever surface happens to draw it. The desktop used
/// to carry its own `>= 60` in a JSX ternary while the reports made no
/// judgement at all, which meant the same estate could look fine in print and
/// amber in the explorer.
pub const HEALTHY_TAG_COVERAGE_PERCENT: u32 = 60;

/// A resource group with more than this share of its resources missing a
/// required tag is called out rather than merely listed.
pub const FLAGGED_NON_COMPLIANT_SHARE: f64 = 0.5;

/// Is coverage high enough to read as healthy?
pub fn is_coverage_healthy(percent: u32) -> bool {
    percent >= HEALTHY_TAG_COVERAGE_PERCENT
}

/// Is a group's non-compliance high enough to call out?
///
/// Strictly greater than the share, so an even split is not yet flagged, and
/// an empty group never is.
pub fn is_group_flagged(non_compliant: usize, resources: usize) -> bool {
    resources > 0 && non_compliant as f64 > resources as f64 * FLAGGED_NON_COMPLIANT_SHARE
}

/// Whole-percent share, truncated, and zero rather than a division by zero.
///
/// Every percentage the app shows goes through this. The explorer used to
/// round its own while the report truncated, so the same estate could read
/// 60% (healthy) in one and 59% (amber) in the other.
pub fn percent(part: usize, whole: usize) -> u32 {
    (part * 100).checked_div(whole).unwrap_or_default() as u32
}

/// The tag keys on a resource. Absent tags and `{}` both mean untagged.
pub(crate) fn tag_keys(resource: &Resource) -> impl Iterator<Item = &str> {
    resource
        .tags
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|tags| tags.keys().map(String::as_str))
}

/// How much of the estate carries any tag at all.
#[derive(Debug, Serialize)]
pub struct TagCoverage {
    pub tagged: usize,
    pub untagged: usize,
    pub percent: u32,
}

impl TagCoverage {
    /// Is coverage at or above [`HEALTHY_TAG_COVERAGE_PERCENT`]?
    pub fn is_healthy(&self) -> bool {
        is_coverage_healthy(self.percent)
    }
}

/// The governance picture, with the thresholds already applied.
#[derive(Debug, Serialize)]
pub struct GovernanceAnalysis {
    /// How many distinct tag keys the estate uses.
    pub distinct_keys: usize,
    /// The most-used keys, busiest first.
    pub top_keys: Vec<TagKeyCoverage>,
    /// How many keys `top_keys` was cut from, so a surface can say so.
    pub top_keys_total: usize,
    /// Coverage per subscription that holds resources, by name.
    pub subscriptions: Vec<SubscriptionCoverage>,
    /// Resources missing at least one required tag.
    pub non_compliant: usize,
    /// The groups holding most of them, worst first.
    pub worst_groups: Vec<GroupCompliance>,
    /// How many groups with misses `worst_groups` was cut from.
    pub worst_groups_total: usize,
}

impl GovernanceAnalysis {
    /// Did this snapshot record any required-tag misses? A snapshot collected
    /// with no `required_tags` configured and one that is fully compliant both
    /// answer no, so neither surface claims a compliance sweep happened.
    pub fn has_compliance_data(&self) -> bool {
        self.non_compliant > 0
    }
}

#[derive(Debug, Serialize)]
pub struct TagKeyCoverage {
    pub key: String,
    pub count: usize,
    /// Share of the *tagged* resources carrying this key.
    pub percent: u32,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionCoverage {
    pub subscription_id: String,
    pub display_name: String,
    pub percent: u32,
    /// [`is_coverage_healthy`] applied, so no surface repeats the comparison.
    pub healthy: bool,
}

#[derive(Debug, Serialize)]
pub struct GroupCompliance {
    pub name: String,
    pub subscription_name: String,
    /// Every resource in the group, not just the offenders.
    pub resources: usize,
    pub non_compliant: usize,
    /// Which required tags were missed anywhere in the group, sorted.
    pub missed_tags: Vec<String>,
    /// [`is_group_flagged`] applied.
    pub flagged: bool,
}

/// Group resources are bucketed by, keyed on what the store actually holds:
/// the subscription id and the lowercased group name (or none).
type GroupKey<'a> = (&'a str, Option<&'a str>);

pub fn analyse(
    subscriptions: &[Subscription],
    resource_groups: &[ResourceGroup],
    resources: &[Resource],
    findings: &[Finding],
) -> GovernanceAnalysis {
    let subscription_names: BTreeMap<&str, &str> = subscriptions
        .iter()
        .map(|sub| (sub.subscription_id.as_str(), sub.display_name.as_str()))
        .collect();
    // Group rows keep the original casing; resource rows carry the lowercased
    // join key, so the table can show `rg-App` rather than `rg-app`.
    let group_names: BTreeMap<(&str, String), &str> = resource_groups
        .iter()
        .map(|rg| {
            (
                (rg.subscription_id.as_str(), rg.name.to_lowercase()),
                rg.name.as_str(),
            )
        })
        .collect();

    let mut key_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut tagged = 0usize;
    let mut per_subscription: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut group_sizes: BTreeMap<GroupKey<'_>, usize> = BTreeMap::new();
    for resource in resources {
        let mut has_tags = false;
        for key in tag_keys(resource) {
            has_tags = true;
            *key_counts.entry(key).or_default() += 1;
        }
        if has_tags {
            tagged += 1;
        }
        let totals = per_subscription
            .entry(resource.subscription_id.as_str())
            .or_default();
        totals.0 += 1;
        totals.1 += usize::from(has_tags);
        *group_sizes.entry(group_key(resource)).or_default() += 1;
    }

    let mut top_keys: Vec<TagKeyCoverage> = key_counts
        .iter()
        .map(|(key, count)| TagKeyCoverage {
            key: (*key).to_owned(),
            count: *count,
            percent: percent(*count, tagged),
        })
        .collect();
    // Busiest first, then by key so ties are stable.
    top_keys.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    let top_keys_total = top_keys.len();
    top_keys.truncate(TOP_TAG_KEYS);

    let mut subscription_coverage: Vec<SubscriptionCoverage> = per_subscription
        .into_iter()
        .map(|(id, (total, tagged))| {
            let share = percent(tagged, total);
            SubscriptionCoverage {
                subscription_id: id.to_owned(),
                display_name: subscription_names
                    .get(id)
                    .map_or_else(|| id.to_owned(), |name| (*name).to_owned()),
                percent: share,
                healthy: is_coverage_healthy(share),
            }
        })
        .collect();
    subscription_coverage.sort_by(|a, b| {
        a.display_name
            .cmp(&b.display_name)
            .then_with(|| a.subscription_id.cmp(&b.subscription_id))
    });

    // The audit already decided who is non-compliant and which tags they
    // missed; this only has to attribute that to a group.
    let by_id: BTreeMap<&str, &Resource> = resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource))
        .collect();
    let mut offenders: BTreeMap<GroupKey<'_>, (usize, Vec<String>)> = BTreeMap::new();
    let mut non_compliant = 0usize;
    for finding in findings.iter().filter(|f| f.query_name == TAG_AUDIT) {
        let Some(resource) = finding
            .resource_id
            .as_deref()
            .and_then(|id| by_id.get(id).copied())
        else {
            continue;
        };
        non_compliant += 1;
        let entry = offenders.entry(group_key(resource)).or_default();
        entry.0 += 1;
        for tag in missed_tags(finding) {
            if !entry.1.iter().any(|seen| seen == tag) {
                entry.1.push(tag.to_owned());
            }
        }
    }

    let mut worst_groups: Vec<GroupCompliance> = offenders
        .into_iter()
        .map(|(key, (count, mut missed))| {
            missed.sort();
            let size = group_sizes.get(&key).copied().unwrap_or(count);
            GroupCompliance {
                name: key
                    .1
                    .map(|name| {
                        group_names
                            .get(&(key.0, name.to_owned()))
                            .copied()
                            .unwrap_or(name)
                            .to_owned()
                    })
                    .unwrap_or_else(|| UNGROUPED.to_owned()),
                subscription_name: subscription_names
                    .get(key.0)
                    .map_or_else(|| key.0.to_owned(), |name| (*name).to_owned()),
                resources: size,
                non_compliant: count,
                missed_tags: missed,
                flagged: is_group_flagged(count, size),
            }
        })
        .collect();
    worst_groups.sort_by(|a, b| {
        b.non_compliant
            .cmp(&a.non_compliant)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.subscription_name.cmp(&b.subscription_name))
    });
    let worst_groups_total = worst_groups.len();
    worst_groups.truncate(WORST_GROUPS);

    GovernanceAnalysis {
        distinct_keys: key_counts.len(),
        top_keys,
        top_keys_total,
        subscriptions: subscription_coverage,
        non_compliant,
        worst_groups,
        worst_groups_total,
    }
}

fn group_key(resource: &Resource) -> GroupKey<'_> {
    (
        resource.subscription_id.as_str(),
        resource.resource_group.as_deref(),
    )
}

/// The tags the audit recorded as missing. A finding whose detail has been
/// reshaped still counts as non-compliant; it just names no tags.
fn missed_tags(finding: &Finding) -> impl Iterator<Item = &str> {
    finding
        .detail
        .as_ref()
        .and_then(|detail| detail.get("missing"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::collect::audit;
    use crate::model::normalize_arm_id;

    fn coverage(percent: u32) -> TagCoverage {
        TagCoverage {
            tagged: 0,
            untagged: 0,
            percent,
        }
    }

    fn subscription(id: &str, name: &str) -> Subscription {
        Subscription {
            subscription_id: id.to_owned(),
            display_name: name.to_owned(),
            state: None,
            tags: None,
        }
    }

    fn group(subscription_id: &str, name: &str) -> ResourceGroup {
        ResourceGroup {
            id: normalize_arm_id(&format!("/subscriptions/{subscription_id}/rg/{name}")),
            name: name.to_owned(),
            subscription_id: subscription_id.to_owned(),
            location: None,
            tags: None,
        }
    }

    fn resource(subscription_id: &str, group: Option<&str>, name: &str, tags: Value) -> Resource {
        Resource {
            id: normalize_arm_id(&format!(
                "/subscriptions/{subscription_id}/{}/{name}",
                group.unwrap_or("none")
            )),
            display_id: name.to_owned(),
            name: name.to_owned(),
            azure_type: "microsoft.web/sites".to_owned(),
            kind: None,
            location: None,
            resource_group: group.map(str::to_lowercase),
            subscription_id: subscription_id.to_owned(),
            tags: if tags.is_null() { None } else { Some(tags) },
            sku: None,
            identity: None,
            properties: None,
        }
    }

    #[test]
    fn unit_treats_the_threshold_itself_as_healthy_when_judging_coverage() {
        assert!(coverage(HEALTHY_TAG_COVERAGE_PERCENT).is_healthy());
        assert!(coverage(HEALTHY_TAG_COVERAGE_PERCENT + 1).is_healthy());
        assert!(!coverage(HEALTHY_TAG_COVERAGE_PERCENT - 1).is_healthy());
    }

    #[test]
    fn unit_flags_a_group_only_past_the_share_when_judging_compliance() {
        // An even split is not yet flagged.
        assert!(!is_group_flagged(5, 10));
        assert!(is_group_flagged(6, 10));
        assert!(!is_group_flagged(0, 10));
    }

    #[test]
    fn unit_never_flags_an_empty_group_when_judging_compliance() {
        assert!(!is_group_flagged(0, 0));
        // Guard against a divide-by-zero reading as "everything is broken".
        assert!(!is_group_flagged(3, 0));
    }

    #[test]
    fn unit_keeps_the_share_a_proportion_when_read() {
        assert!((0.0..=1.0).contains(&FLAGGED_NON_COMPLIANT_SHARE));
    }

    #[test]
    fn unit_truncates_rather_than_rounds_when_taking_a_percentage() {
        assert_eq!(percent(2, 3), 66);
        assert_eq!(percent(0, 0), 0);
        assert_eq!(percent(3, 3), 100);
    }

    #[test]
    fn unit_counts_an_empty_tag_object_as_untagged_when_reading_keys() {
        let bare = resource("s1", Some("rg"), "r1", json!({}));
        assert_eq!(tag_keys(&bare).count(), 0);
    }

    #[test]
    fn unit_ranks_tag_keys_by_use_when_analysing() {
        let resources = vec![
            resource("s1", Some("rg"), "r1", json!({"env": "prod", "owner": "a"})),
            resource("s1", Some("rg"), "r2", json!({"env": "dev"})),
            resource("s1", Some("rg"), "r3", Value::Null),
        ];

        let analysis = analyse(&[], &[], &resources, &[]);

        assert_eq!(analysis.distinct_keys, 2);
        assert_eq!(analysis.top_keys[0].key, "env");
        assert_eq!(analysis.top_keys[0].count, 2);
        // Two of the two *tagged* resources, not two of the three resources.
        assert_eq!(analysis.top_keys[0].percent, 100);
        assert_eq!(analysis.top_keys[1].percent, 50);
    }

    #[test]
    fn unit_reports_coverage_per_subscription_with_the_verdict_when_analysing() {
        let resources = vec![
            resource("s1", Some("rg"), "r1", json!({"env": "prod"})),
            resource("s1", Some("rg"), "r2", Value::Null),
            resource("s2", Some("rg"), "r3", json!({"env": "prod"})),
        ];
        let subscriptions = vec![subscription("s1", "Production"), subscription("s2", "Dev")];

        let analysis = analyse(&subscriptions, &[], &resources, &[]);

        // Sorted by display name, so Dev comes first.
        assert_eq!(analysis.subscriptions[0].display_name, "Dev");
        assert_eq!(analysis.subscriptions[0].percent, 100);
        assert!(analysis.subscriptions[0].healthy);
        assert_eq!(analysis.subscriptions[1].percent, 50);
        assert!(!analysis.subscriptions[1].healthy);
    }

    #[test]
    fn unit_omits_a_subscription_holding_no_resources_when_analysing() {
        let subscriptions = vec![
            subscription("s1", "Production"),
            subscription("s2", "Empty"),
        ];
        let resources = vec![resource("s1", Some("rg"), "r1", json!({"env": "prod"}))];

        let analysis = analyse(&subscriptions, &[], &resources, &[]);

        assert_eq!(analysis.subscriptions.len(), 1);
        assert_eq!(analysis.subscriptions[0].display_name, "Production");
    }

    #[test]
    fn unit_attributes_recorded_misses_to_their_group_when_analysing() {
        let resources = vec![
            resource(
                "s1",
                Some("rg-app"),
                "r1",
                json!({"env": "p", "owner": "a"}),
            ),
            resource("s1", Some("rg-app"), "r2", json!({"env": "p"})),
            resource("s1", Some("rg-dev"), "r3", Value::Null),
            resource("s1", Some("rg-dev"), "r4", Value::Null),
        ];
        let findings = audit::missing_required_tags(&resources, &["owner".into(), "env".into()]);
        let subscriptions = vec![subscription("s1", "Production")];
        let groups = vec![group("s1", "rg-app"), group("s1", "rg-dev")];

        let analysis = analyse(&subscriptions, &groups, &resources, &findings);

        assert_eq!(analysis.non_compliant, 3);
        let worst = &analysis.worst_groups[0];
        assert_eq!(worst.name, "rg-dev");
        assert_eq!(worst.subscription_name, "Production");
        assert_eq!(worst.non_compliant, 2);
        // The group's size is every resource in it, not just the offenders.
        assert_eq!(worst.resources, 2);
        assert!(worst.flagged);
        // Sorted, and deduplicated across the group's resources.
        assert_eq!(worst.missed_tags, ["env", "owner"]);

        let second = &analysis.worst_groups[1];
        assert_eq!(second.name, "rg-app");
        assert_eq!(second.resources, 2);
        assert_eq!(second.non_compliant, 1);
        assert_eq!(second.missed_tags, ["owner"]);
        // An even split is listed but not called out.
        assert!(!second.flagged);
    }

    #[test]
    fn unit_shows_the_groups_own_casing_when_analysing() {
        let resources = vec![resource("s1", Some("rg-App"), "r1", Value::Null)];
        let findings = audit::missing_required_tags(&resources, &["owner".into()]);

        let analysis = analyse(&[], &[group("s1", "rg-App")], &resources, &findings);

        assert_eq!(analysis.worst_groups[0].name, "rg-App");
    }

    #[test]
    fn unit_records_how_many_keys_the_summary_was_cut_from_when_analysing() {
        let resources: Vec<Resource> = (0..10)
            .map(|i| {
                resource(
                    "s",
                    Some("rg"),
                    &format!("r{i}"),
                    serde_json::json!({ format!("key{i}"): "v" }),
                )
            })
            .collect();
        let analysis = analyse(&[], &[], &resources, &[]);
        assert_eq!(analysis.top_keys.len(), TOP_TAG_KEYS);
        assert_eq!(analysis.top_keys_total, 10);
        assert_eq!(analysis.worst_groups_total, 0);
    }

    #[test]
    fn unit_names_a_group_less_resource_when_analysing() {
        let resources = vec![resource("s1", None, "r1", Value::Null)];
        let findings = audit::missing_required_tags(&resources, &["owner".into()]);

        let analysis = analyse(&[], &[], &resources, &findings);

        assert_eq!(analysis.worst_groups[0].name, UNGROUPED);
    }

    #[test]
    fn unit_ignores_findings_from_other_audits_when_analysing() {
        let resources = vec![resource("s1", Some("rg"), "r1", Value::Null)];
        let findings = vec![Finding {
            query_name: "storage_public_blob_access".to_owned(),
            category: "security".to_owned(),
            severity: crate::model::Severity::High,
            resource_id: Some(resources[0].id.clone()),
            title: "public".to_owned(),
            detail: None,
        }];

        let analysis = analyse(&[], &[], &resources, &findings);

        assert_eq!(analysis.non_compliant, 0);
        assert!(analysis.worst_groups.is_empty());
        assert!(!analysis.has_compliance_data());
    }

    #[test]
    fn unit_still_counts_a_finding_whose_detail_names_no_tags_when_analysing() {
        let resources = vec![resource("s1", Some("rg"), "r1", Value::Null)];
        let findings = vec![Finding {
            query_name: TAG_AUDIT.to_owned(),
            category: "governance".to_owned(),
            severity: crate::model::Severity::Low,
            resource_id: Some(resources[0].id.clone()),
            title: "r1 is missing tags: owner".to_owned(),
            detail: None,
        }];

        let analysis = analyse(&[], &[], &resources, &findings);

        assert_eq!(analysis.non_compliant, 1);
        assert!(analysis.worst_groups[0].missed_tags.is_empty());
    }
}
