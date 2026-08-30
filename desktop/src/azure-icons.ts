export const ALL_RESOURCES_ICON = "/icons/general/10001-icon-service-All-Resources.svg";
export const SUBSCRIPTION_ICON = "/icons/general/10002-icon-service-Subscriptions.svg";
export const RESOURCE_GROUP_ICON = "/icons/general/10007-icon-service-Resource-Groups.svg";
export const VNET_ICON = "/icons/networking/10061-icon-service-Virtual-Networks.svg";

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
