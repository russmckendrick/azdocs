//! Bucketing an estate's resources into their resource groups.
//!
//! ARG can return a resource whose `resourceGroup` has no matching group row —
//! a partial snapshot, or a resource that lives at subscription scope — so the
//! explorer has to synthesise a group rather than drop it. That rule, the join
//! key it uses, and the name it gives a group-less resource were written out
//! twice in Rust (`topology.rs`) and a third time in TypeScript
//! (`topology-model.ts`), with the three disagreeing on the synthetic id and its
//! casing. This module is the one copy.
//!
//! Note this is deliberately *not* the CLI's `report::details::group_key`. That
//! one joins with `/` so `starts_with("{subscription}/")` can select a
//! subscription's pages; this one only ever tests equality, and uses `\0`
//! because no ARM name can contain it.

use std::collections::HashMap;

use azdocs::model::{Resource, ResourceGroup};

/// Join key for "which resource group is this".
///
/// Both halves are lowercased: ARG returns inconsistent casing, and a group
/// row and a resource's `resourceGroup` field routinely disagree on it.
pub fn group_key(subscription_id: &str, group_name: &str) -> String {
    format!(
        "{}\0{}",
        subscription_id.to_lowercase(),
        group_name.to_lowercase()
    )
}

/// One resource group and the resources that landed in it.
#[derive(Debug)]
pub struct GroupBucket<'a> {
    /// Lowercased ARM id, synthesised when the snapshot has no row for it.
    pub id: String,
    pub name: String,
    pub subscription_id: String,
    pub resources: Vec<&'a Resource>,
}

/// Bucket `resources` into `groups`, synthesising a bucket for any
/// (subscription, group) pair that has resources but no row.
///
/// Returns the buckets in a stable order — declared groups first in their
/// snapshot order, then synthetic ones as they are encountered — plus a
/// resource id → bucket index map so callers can look up a resource's home
/// without rescanning.
///
/// `subscriptions` filters to those subscription ids when non-empty.
///
/// `scope_name` is the displayed stand-in group name for a resource that
/// carries no resource group (`common.subscription_scope` in the labels). It
/// reaches the relationship map as a group name, so it is a label; casing is
/// safe because the join key lowercases both halves and the synthetic id
/// lowercases the name.
pub fn bucket_resources<'a>(
    groups: &[ResourceGroup],
    resources: impl IntoIterator<Item = &'a Resource>,
    subscriptions: &[String],
    scope_name: &str,
) -> (Vec<GroupBucket<'a>>, HashMap<String, usize>) {
    let mut buckets: Vec<GroupBucket<'a>> = Vec::new();
    let mut index_by_key: HashMap<String, usize> = HashMap::new();

    for group in groups {
        if !subscriptions.is_empty() && !subscriptions.contains(&group.subscription_id) {
            continue;
        }
        let key = group_key(&group.subscription_id, &group.name);
        if index_by_key.contains_key(&key) {
            continue;
        }
        index_by_key.insert(key, buckets.len());
        buckets.push(GroupBucket {
            id: group.id.clone(),
            name: group.name.clone(),
            subscription_id: group.subscription_id.clone(),
            resources: Vec::new(),
        });
    }

    let mut index_by_resource: HashMap<String, usize> = HashMap::new();
    for resource in resources {
        let name = resource
            .resource_group
            .clone()
            .unwrap_or_else(|| scope_name.to_owned());
        let key = group_key(&resource.subscription_id, &name);
        let index = *index_by_key.entry(key).or_insert_with(|| {
            buckets.push(GroupBucket {
                id: format!(
                    "/subscriptions/{}/resourcegroups/{}",
                    resource.subscription_id.to_lowercase(),
                    name.to_lowercase()
                ),
                name,
                subscription_id: resource.subscription_id.clone(),
                resources: Vec::new(),
            });
            buckets.len() - 1
        });
        index_by_resource.insert(resource.id.clone(), index);
        buckets[index].resources.push(resource);
    }

    (buckets, index_by_resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCOPE: &str = "Subscription scope";

    fn group(sub: &str, name: &str) -> ResourceGroup {
        ResourceGroup {
            id: format!("/subscriptions/{sub}/resourcegroups/{name}"),
            name: name.to_owned(),
            subscription_id: sub.to_owned(),
            location: None,
            tags: None,
        }
    }

    fn resource(sub: &str, rg: Option<&str>, name: &str) -> Resource {
        Resource {
            id: format!("/subscriptions/{sub}/providers/x/{name}"),
            display_id: format!("/subscriptions/{sub}/providers/x/{name}"),
            name: name.to_owned(),
            azure_type: "microsoft.compute/virtualmachines".to_owned(),
            kind: None,
            location: None,
            resource_group: rg.map(str::to_owned),
            subscription_id: sub.to_owned(),
            tags: None,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    #[test]
    fn unit_places_resources_in_their_declared_group_when_a_row_exists() {
        let groups = vec![group("s1", "rg-app")];
        let resources = vec![resource("s1", Some("rg-app"), "vm-1")];

        let (buckets, by_resource) = bucket_resources(&groups, &resources, &[], SCOPE);

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].id, "/subscriptions/s1/resourcegroups/rg-app");
        assert_eq!(buckets[0].resources.len(), 1);
        assert_eq!(by_resource[&resources[0].id], 0);
    }

    #[test]
    fn unit_matches_across_casing_when_arg_disagrees_with_itself() {
        // The group row says "RG-App"; the resource says "rg-app".
        let groups = vec![group("S1", "RG-App")];
        let resources = vec![resource("s1", Some("rg-app"), "vm-1")];

        let (buckets, _) = bucket_resources(&groups, &resources, &[], SCOPE);

        assert_eq!(buckets.len(), 1, "casing must not split one group in two");
        assert_eq!(buckets[0].resources.len(), 1);
    }

    #[test]
    fn unit_synthesises_a_group_when_the_snapshot_has_no_row_for_it() {
        let resources = vec![resource("s1", Some("rg-orphan"), "vm-1")];

        let (buckets, _) = bucket_resources(&[], &resources, &[], SCOPE);

        assert_eq!(buckets.len(), 1);
        assert_eq!(
            buckets[0].id, "/subscriptions/s1/resourcegroups/rg-orphan",
            "a synthesised id must still be a lowercased ARM id"
        );
    }

    #[test]
    fn unit_gathers_group_less_resources_under_subscription_scope() {
        let resources = vec![resource("s1", None, "vm-1"), resource("s1", None, "vm-2")];

        let (buckets, _) = bucket_resources(&[], &resources, &[], SCOPE);

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name, SCOPE);
        assert_eq!(buckets[0].resources.len(), 2);
    }

    #[test]
    fn unit_drops_groups_outside_the_requested_subscriptions() {
        let groups = vec![group("s1", "rg-app"), group("s2", "rg-dev")];
        let resources = vec![resource("s1", Some("rg-app"), "vm-1")];

        let (buckets, _) = bucket_resources(&groups, &resources, &["s1".to_owned()], SCOPE);

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].subscription_id, "s1");
    }

    #[test]
    fn unit_places_every_resource_in_exactly_one_bucket_when_bucketing() {
        // The frontend derives "which group is this resource in" from these
        // buckets, so a resource appearing twice would duplicate it on the map
        // and a resource appearing nowhere would silently vanish.
        let groups = vec![group("s1", "rg-app")];
        let resources = vec![
            resource("s1", Some("rg-app"), "vm-1"),
            resource("s1", Some("rg-missing"), "vm-2"),
            resource("s1", None, "vm-3"),
            resource("s2", Some("rg-other"), "vm-4"),
        ];

        let (buckets, by_resource) = bucket_resources(&groups, &resources, &[], SCOPE);

        let placed: usize = buckets.iter().map(|b| b.resources.len()).sum();
        assert_eq!(placed, resources.len(), "every resource is placed");
        assert_eq!(
            by_resource.len(),
            resources.len(),
            "every resource is indexed"
        );
        for resource in &resources {
            let index = by_resource[&resource.id];
            assert!(
                buckets[index].resources.iter().any(|r| r.id == resource.id),
                "the index must point at the bucket actually holding {}",
                resource.name
            );
        }
    }

    #[test]
    fn unit_keeps_declared_groups_that_hold_no_resources() {
        let groups = vec![group("s1", "rg-empty")];

        let (buckets, _) = bucket_resources(&groups, &[], &[], SCOPE);

        assert_eq!(buckets.len(), 1, "an empty group is still a group");
        assert!(buckets[0].resources.is_empty());
    }
}
