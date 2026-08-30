import { preciseDateTime, resourceName } from "../format";

type DataRecord = Record<string, unknown>;

type FlatField = {
  path: string[];
  value: unknown;
};

type DataCollection = {
  path: string[];
  items: unknown[];
};

const ACRONYMS = new Map([
  ["api", "API"],
  ["arm", "ARM"],
  ["cidr", "CIDR"],
  ["cpu", "CPU"],
  ["dns", "DNS"],
  ["gb", "GB"],
  ["id", "ID"],
  ["ids", "IDs"],
  ["ip", "IP"],
  ["nsg", "NSG"],
  ["os", "OS"],
  ["sku", "SKU"],
  ["ssl", "SSL"],
  ["tls", "TLS"],
  ["uri", "URI"],
  ["url", "URL"],
  ["vm", "VM"],
  ["vnet", "VNet"],
]);

const ISO_DATE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})$/;
const IDENTIFIER_KEY = /(?:^|\b)(?:id|ids|identifier|uri|url|path|scope)$/i;
const IDENTIFIER_VALUE = /^(?:\/subscriptions\/|https?:\/\/|[0-9a-f]{8}-[0-9a-f-]{27,})/i;
const STATE_VALUE = /^(?:allow|allowed|approved|attached|connected|default|deny|denied|disabled|enabled|failed|running|static|stopped|succeeded)$/i;

function isRecord(value: unknown): value is DataRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isScalar(value: unknown) {
  return !Array.isArray(value) && !isRecord(value);
}

function isScalarArray(value: unknown): value is unknown[] {
  return Array.isArray(value) && value.every(isScalar);
}

export function hasStoredValue(value: unknown) {
  if (value === undefined || value === null) return false;
  if (Array.isArray(value)) return value.length > 0;
  if (isRecord(value)) return Object.keys(value).length > 0;
  return true;
}

export function describeStoredValue(value: unknown) {
  if (!hasStoredValue(value)) return "No value stored";
  if (Array.isArray(value)) return `${value.length} ${value.length === 1 ? "item" : "items"}`;
  if (isRecord(value)) {
    const count = Object.keys(value).length;
    return `${count} ${count === 1 ? "field" : "fields"}`;
  }
  return "Stored value";
}

function humanizeKey(key: string) {
  const words = key
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .replace(/([a-z\d])([A-Z])/g, "$1 $2")
    .replace(/[_.-]+/g, " ")
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => ACRONYMS.get(word.toLowerCase()) ?? word.toLowerCase());

  if (words.length === 0) return key;
  const [first, ...rest] = words;
  const leading = ACRONYMS.has(first.toLowerCase())
    ? first
    : `${first.charAt(0).toUpperCase()}${first.slice(1)}`;
  return [leading, ...rest].join(" ");
}

function readablePath(path: string[]) {
  return path.map(humanizeKey).join(" / ");
}

function fieldKey(path: string[]) {
  return path.join("\u001f");
}

function looksLikeIdentifier(path: string[], value: string) {
  return IDENTIFIER_KEY.test(path.map(humanizeKey).join(" ")) || IDENTIFIER_VALUE.test(value);
}


function ScalarValue({ path = [], value }: { path?: string[]; value: unknown }) {
  if (value === undefined || value === null) {
    return <span className="adaptive-value empty">Not stored</span>;
  }

  if (Array.isArray(value)) {
    if (value.length === 0) return <span className="adaptive-value empty">No items stored</span>;
    return (
      <span className="adaptive-inline-list">
        {value.map((item, index) => <ScalarValue key={index} path={path} value={item} />)}
      </span>
    );
  }

  if (isRecord(value)) return <span className="adaptive-value empty">No fields stored</span>;
  if (typeof value === "boolean") return <span className="adaptive-value boolean">{value ? "True" : "False"}</span>;
  if (typeof value === "number") return <span className="adaptive-value number">{value.toLocaleString()}</span>;

  const text = String(value);
  if (text.length === 0) return <span className="adaptive-value empty">Empty string</span>;

  if (ISO_DATE.test(text)) {
    const formatted = preciseDateTime(text);
    if (formatted) {
      return (
        <span className="adaptive-date">
          <time dateTime={text}>{formatted}</time>
          <code>{text}</code>
        </span>
      );
    }
  }

  const identifier = looksLikeIdentifier(path, text);
  const state = STATE_VALUE.test(text);
  return (
    <span className={`adaptive-value${identifier ? " identifier" : ""}${state ? " state" : ""}`}>
      {text}
    </span>
  );
}

function collectRecord(
  value: DataRecord,
  prefix: string[],
  fields: FlatField[],
  collections: DataCollection[],
) {
  for (const [key, item] of Object.entries(value)) {
    const path = [...prefix, key];
    if (Array.isArray(item)) {
      if (item.length > 0 && !isScalarArray(item)) collections.push({ path, items: item });
      else fields.push({ path, value: item });
    } else if (isRecord(item)) {
      if (Object.keys(item).length === 0) fields.push({ path, value: item });
      else collectRecord(item, path, fields, collections);
    } else {
      fields.push({ path, value: item });
    }
  }
}

function flattenCollectionItem(value: unknown, path: string[] = [], fields: FlatField[] = []) {
  if (Array.isArray(value)) {
    if (value.length === 0 || isScalarArray(value)) fields.push({ path: path.length > 0 ? path : ["Value"], value });
    else value.forEach((item, index) => flattenCollectionItem(item, [...path, `Item ${index + 1}`], fields));
  } else if (isRecord(value)) {
    const entries = Object.entries(value);
    if (entries.length === 0) fields.push({ path: path.length > 0 ? path : ["Value"], value });
    else entries.forEach(([key, item]) => flattenCollectionItem(item, [...path, key], fields));
  } else {
    fields.push({ path: path.length > 0 ? path : ["Value"], value });
  }
  return fields;
}

function FieldName({ path }: { path: string[] }) {
  const context = path.slice(0, -1);
  return (
    <>
      {context.length > 0 ? <small>{readablePath(context)}</small> : null}
      <span>{humanizeKey(path.at(-1) ?? "Value")}</span>
    </>
  );
}

function isWideField(field: FlatField) {
  if (Array.isArray(field.value)) {
    return field.value.length > 3 || field.value.map(String).join(", ").length > 56;
  }
  if (typeof field.value !== "string") return false;
  return field.value.length > 56 || looksLikeIdentifier(field.path, field.value);
}

function propertyRows(fields: FlatField[]) {
  const rows: FlatField[][] = [];
  let pending: FlatField[] = [];
  for (const field of fields) {
    if (isWideField(field)) {
      if (pending.length > 0) rows.push(pending);
      rows.push([field]);
      pending = [];
    } else {
      pending.push(field);
      if (pending.length === 2) {
        rows.push(pending);
        pending = [];
      }
    }
  }
  if (pending.length > 0) rows.push(pending);
  return rows;
}

function PropertyTable({ fields }: { fields: FlatField[] }) {
  if (fields.length === 0) return null;
  return (
    <table className="adaptive-property-table">
      <colgroup><col /><col /><col /><col /></colgroup>
      <tbody>
        {propertyRows(fields).map((row) => (
          <tr key={row.map((field) => fieldKey(field.path)).join("|")}>
            {row.map((field) => <FragmentField field={field} key={fieldKey(field.path)} wide={row.length === 1} />)}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function FragmentField({ field, wide }: { field: FlatField; wide: boolean }) {
  return (
    <>
      <th scope="row"><FieldName path={field.path} /></th>
      <td colSpan={wide ? 3 : 1}><ScalarValue path={field.path} value={field.value} /></td>
    </>
  );
}

function sequenceTitle(value: unknown, index: number) {
  if (isRecord(value)) {
    for (const key of ["displayName", "name", "label", "id", "type"]) {
      const candidate = value[key];
      if (typeof candidate === "string" && candidate.length > 0) {
        if (key === "id" && candidate.includes("/")) return resourceName(candidate);
        return candidate;
      }
    }
  }
  return `Item ${index + 1}`;
}

function CollectionTable({ collection }: { collection: DataCollection }) {
  const rows = collection.items.map((item, index) => ({
    label: sequenceTitle(item, index),
    fields: flattenCollectionItem(item),
  }));
  const columnMap = new Map<string, string[]>();
  rows.forEach((row) => row.fields.forEach((field) => columnMap.set(fieldKey(field.path), field.path)));
  const columns = Array.from(columnMap.entries());
  const horizontal = columns.length > 0 && columns.length <= 6;

  return (
    <div className="adaptive-collection-table-wrap">
      <table className={horizontal ? "adaptive-collection-table" : "adaptive-collection-table vertical"}>
        <caption><span>{readablePath(collection.path)}</span><small>{describeStoredValue(collection.items)}</small></caption>
        {horizontal ? (
          <>
            <thead><tr><th>Item</th>{columns.map(([key, path]) => <th key={key}><FieldName path={path} /></th>)}</tr></thead>
            <tbody>
              {rows.map((row, index) => {
                const values = new Map(row.fields.map((field) => [fieldKey(field.path), field]));
                return (
                  <tr key={index}>
                    <th scope="row">{row.label}</th>
                    {columns.map(([key, path]) => <td key={key}><ScalarValue path={path} value={values.get(key)?.value} /></td>)}
                  </tr>
                );
              })}
            </tbody>
          </>
        ) : (
          <>
            <thead><tr><th>Item</th><th>Field</th><th>Stored value</th></tr></thead>
            <tbody>
              {rows.flatMap((row, itemIndex) => row.fields.map((field, fieldIndex) => (
                <tr className={fieldIndex === 0 ? "item-start" : undefined} key={`${itemIndex}-${fieldKey(field.path)}`}>
                  {fieldIndex === 0 ? <th scope="rowgroup" rowSpan={row.fields.length}>{row.label}</th> : null}
                  <th scope="row"><FieldName path={field.path} /></th>
                  <td><ScalarValue path={field.path} value={field.value} /></td>
                </tr>
              )))}
            </tbody>
          </>
        )}
      </table>
    </div>
  );
}

export function AdaptiveDataView({ value }: { value: unknown }) {
  const fields: FlatField[] = [];
  const collections: DataCollection[] = [];

  if (isRecord(value)) collectRecord(value, [], fields, collections);
  else if (Array.isArray(value) && !isScalarArray(value)) collections.push({ path: ["Items"], items: value });
  else fields.push({ path: ["Value"], value });

  return (
    <div className="adaptive-data">
      <PropertyTable fields={fields} />
      {collections.map((collection) => <CollectionTable collection={collection} key={fieldKey(collection.path)} />)}
    </div>
  );
}
