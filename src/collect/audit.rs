//! Post-collection audits computed in Rust rather than KQL, because they need
//! configuration (e.g. the required tag list) that queries can't see.

use serde_json::{Value, json};

use crate::config::AuditConfig;
use crate::model::{Finding, Resource, ResourceGroup, Severity, Subscription};

pub const REQUIRED_TAGS_QUERY: &str = "missing_required_tags";

/// One finding per resource that is missing at least one required tag.
pub fn missing_required_tags(resources: &[Resource], required: &[String]) -> Vec<Finding> {
    if required.is_empty() {
        return Vec::new();
    }
    resources
        .iter()
        .filter_map(|resource| {
            tag_finding(
                resource.tags.as_ref(),
                required,
                &resource.id,
                &resource.name,
                &resource.display_id,
                &resource.azure_type,
            )
        })
        .collect()
}

/// The same check for resource groups. The finding's resource id is the
/// group's lowercased ARM id, which no `resources` row carries, so reports
/// list it as an unresolved (estate-level) finding rather than mis-attributing it.
pub fn missing_required_tags_on_groups(
    groups: &[ResourceGroup],
    required: &[String],
) -> Vec<Finding> {
    if required.is_empty() {
        return Vec::new();
    }
    groups
        .iter()
        .filter_map(|group| {
            tag_finding(
                group.tags.as_ref(),
                required,
                &group.id,
                &group.name,
                &group.id,
                "microsoft.resources/subscriptions/resourcegroups",
            )
        })
        .collect()
}

/// The same check for subscriptions, keyed by `/subscriptions/<id>`.
pub fn missing_required_tags_on_subscriptions(
    subscriptions: &[Subscription],
    required: &[String],
) -> Vec<Finding> {
    if required.is_empty() {
        return Vec::new();
    }
    subscriptions
        .iter()
        .filter_map(|subscription| {
            let id = format!(
                "/subscriptions/{}",
                subscription.subscription_id.to_lowercase()
            );
            tag_finding(
                subscription.tags.as_ref(),
                required,
                &id,
                &subscription.display_name,
                &id,
                "microsoft.resources/subscriptions",
            )
        })
        .collect()
}

/// Every required-tag finding the audit configuration asks for.
pub fn required_tag_findings(
    config: &AuditConfig,
    resources: &[Resource],
    groups: &[ResourceGroup],
    subscriptions: &[Subscription],
) -> Vec<Finding> {
    let required = &config.required_tags;
    let mut findings = missing_required_tags(resources, required);
    if config.tag_resource_groups {
        findings.extend(missing_required_tags_on_groups(groups, required));
    }
    if config.tag_subscriptions {
        findings.extend(missing_required_tags_on_subscriptions(
            subscriptions,
            required,
        ));
    }
    findings
}

fn tag_finding(
    tags: Option<&Value>,
    required: &[String],
    id: &str,
    name: &str,
    display_id: &str,
    azure_type: &str,
) -> Option<Finding> {
    let missing: Vec<&str> = required
        .iter()
        .filter(|tag| !has_tag(tags, tag))
        .map(String::as_str)
        .collect();
    if missing.is_empty() {
        return None;
    }
    Some(Finding {
        query_name: REQUIRED_TAGS_QUERY.to_owned(),
        category: "governance".to_owned(),
        severity: Severity::Low,
        resource_id: Some(id.to_owned()),
        title: format!("{name} is missing tags: {}", missing.join(", ")),
        detail: Some(json!({
            "id": display_id,
            "type": azure_type,
            "missing": missing,
        })),
    })
}

/// Tag names in Azure are case-insensitive.
fn has_tag(tags: Option<&Value>, tag: &str) -> bool {
    tags.and_then(|tags| tags.as_object())
        .is_some_and(|tags| tags.keys().any(|key| key.eq_ignore_ascii_case(tag)))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::normalize_arm_id;

    fn resource_with_tags(tags: Option<serde_json::Value>) -> Resource {
        Resource {
            id: normalize_arm_id("/s1/rg/r1"),
            display_id: "/s1/rg/r1".to_owned(),
            name: "r1".to_owned(),
            azure_type: "microsoft.web/sites".to_owned(),
            kind: None,
            location: None,
            resource_group: Some("rg".to_owned()),
            subscription_id: "s1".to_owned(),
            tags,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    #[test]
    fn unit_flags_resource_groups_missing_tags_when_enabled() {
        let groups = vec![ResourceGroup {
            id: "/subscriptions/s1/resourcegroups/rg-app".into(),
            name: "rg-app".into(),
            subscription_id: "s1".into(),
            location: None,
            tags: Some(json!({"env": "prod"})),
        }];
        let config = AuditConfig {
            required_tags: vec!["env".into(), "owner".into()],
            ..AuditConfig::default()
        };

        let findings = required_tag_findings(&config, &[], &groups, &[]);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].resource_id.as_deref(),
            Some("/subscriptions/s1/resourcegroups/rg-app")
        );
        assert_eq!(findings[0].title, "rg-app is missing tags: owner");
    }

    #[test]
    fn unit_skips_subscriptions_when_disabled() {
        let subscriptions = vec![Subscription {
            subscription_id: "S1".into(),
            display_name: "Prod".into(),
            state: None,
            tags: None,
        }];
        let mut config = AuditConfig {
            required_tags: vec!["owner".into()],
            ..AuditConfig::default()
        };

        assert!(required_tag_findings(&config, &[], &[], &subscriptions).is_empty());
        config.tag_subscriptions = true;
        let findings = required_tag_findings(&config, &[], &[], &subscriptions);
        assert_eq!(
            findings[0].resource_id.as_deref(),
            Some("/subscriptions/s1")
        );
    }

    #[test]
    fn flags_resources_missing_a_required_tag() {
        let resources = vec![resource_with_tags(Some(json!({"env": "prod"})))];

        let findings = missing_required_tags(&resources, &["env".into(), "owner".into()]);

        assert_eq!(findings[0].title, "r1 is missing tags: owner");
    }

    #[test]
    fn tag_comparison_is_case_insensitive() {
        let resources = vec![resource_with_tags(Some(json!({"Owner": "russ"})))];

        let findings = missing_required_tags(&resources, &["owner".into()]);

        assert!(findings.is_empty());
    }

    #[test]
    fn untagged_resources_miss_everything() {
        let resources = vec![resource_with_tags(None)];

        let findings = missing_required_tags(&resources, &["owner".into()]);

        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn empty_required_list_produces_no_findings() {
        let resources = vec![resource_with_tags(None)];

        let findings = missing_required_tags(&resources, &[]);

        assert!(findings.is_empty());
    }
}
