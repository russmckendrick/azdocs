export const ALL_RESOURCES_ICON = "/icons/general/10001-icon-service-All-Resources.svg";
export const SUBSCRIPTION_ICON = "/icons/general/10002-icon-service-Subscriptions.svg";
export const RESOURCE_GROUP_ICON = "/icons/general/10007-icon-service-Resource-Groups.svg";
export const VNET_ICON = "/icons/networking/10061-icon-service-Virtual-Networks.svg";
export const AUDIT_ICON = "/icons/management + governance/00011-icon-service-Compliance.svg";
export const RELATIONSHIPS_ICON = "/icons/management + governance/10318-icon-service-Resource-Graph-Explorer.svg";
export const TAGS_ICON = "/icons/general/10842-icon-service-Tags.svg";
export const LOCATION_ICON = "/icons/general/10818-icon-service-Location.svg";

/**
 * The icon for a resource type, falling back to the generic glyph.
 *
 * Five call sites wrote `type?.icon ?? ALL_RESOURCES_ICON`. The icon itself is
 * a base64 data URI the Rust side puts on `ResourceType`; the fallback is a
 * path served from the vendored catalogue.
 */
export function resourceIcon(type: { icon: string } | undefined) {
  return type?.icon ?? ALL_RESOURCES_ICON;
}

export const SIDEBAR_ICONS = {
  overview: "/icons/general/10015-icon-service-Dashboard.svg",
  estate: ALL_RESOURCES_ICON,
  topology: RELATIONSHIPS_ICON,
  inventory: "/icons/general/10841-icon-service-Table.svg",
  findings: AUDIT_ICON,
  governance: TAGS_ICON,
  history: "/icons/monitor/00563-icon-service-Change-Analysis.svg",
  exports: "/icons/general/10797-icon-service-Download.svg",
  settings: "/icons/general/10805-icon-service-Gear.svg",
};
