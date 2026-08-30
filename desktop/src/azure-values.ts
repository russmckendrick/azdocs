import type { AzureMetadata } from "./types";

// `null` is in these signatures because it is what the wire actually carries:
// a Rust `Option<String>` serialises to `null`, not an absent key. The
// hand-written types used to claim `?: string`, which typed it as `undefined`.
export function displayLocation(
  metadata: AzureMetadata,
  location: string | null | undefined,
  missing = "Global",
) {
  if (!location) return missing;
  return metadata.locations[location.toLowerCase()] ?? location;
}

export function displayKind(
  metadata: AzureMetadata,
  azureType: string,
  kind: string | null | undefined,
  missing = "Default",
) {
  if (!kind) return missing;
  const normalizedKind = kind.toLowerCase();
  const exact = `${azureType.toLowerCase()}:${normalizedKind}`;
  return metadata.kinds[exact] ?? metadata.kinds[`*:${normalizedKind}`] ?? humanizeIdentifier(kind);
}

function humanizeIdentifier(value: string) {
  return value
    .replaceAll(/[_-]+/g, " ")
    .replaceAll(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replaceAll(/([A-Z])([A-Z][a-z])/g, "$1 $2")
    .replaceAll(/,\s*/g, ", ")
    .trim();
}
