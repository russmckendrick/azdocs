//! Post-collection audits computed in Rust rather than KQL, because they need
//! configuration (e.g. the required tag list) that queries can't see.

use serde_json::json;

use crate::model::{Finding, Resource, Severity};

/// One finding per resource that is missing at least one required tag.
pub fn missing_required_tags(resources: &[Resource], required: &[String]) -> Vec<Finding> {
    if required.is_empty() {
        return Vec::new();
    }
    resources
        .iter()
        .filter_map(|resource| {
            let missing: Vec<&str> = required
                .iter()
                .filter(|tag| !has_tag(resource, tag))
                .map(String::as_str)
                .collect();
            if missing.is_empty() {
                return None;
            }
            Some(Finding {
                query_name: "missing_required_tags".to_owned(),
                category: "governance".to_owned(),
                severity: Severity::Low,
                resource_id: Some(resource.id.clone()),
                title: format!("{} is missing tags: {}", resource.name, missing.join(", ")),
                detail: Some(json!({
                    "id": resource.display_id,
                    "type": resource.azure_type,
                    "missing": missing,
                })),
            })
        })
        .collect()
}

/// Tag names in Azure are case-insensitive.
fn has_tag(resource: &Resource, tag: &str) -> bool {
    resource
        .tags
        .as_ref()
        .and_then(|tags| tags.as_object())
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
