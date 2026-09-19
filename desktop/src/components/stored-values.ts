import { plural } from "../format";
import { labels } from "../labels";

const words = () => labels().desktop.data_view;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function hasStoredValue(value: unknown) {
  if (value === undefined || value === null) return false;
  if (Array.isArray(value)) return value.length > 0;
  if (isRecord(value)) return Object.keys(value).length > 0;
  return true;
}

export function describeStoredValue(value: unknown) {
  const text = words();
  if (!hasStoredValue(value)) return text.no_value;
  if (Array.isArray(value)) return plural(text.items, value.length);
  if (isRecord(value)) return plural(text.fields, Object.keys(value).length);
  return text.stored_value;
}
