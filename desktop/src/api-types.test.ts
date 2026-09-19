import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const MODEL = resolve(__dirname, "../../src/model/mod.rs");
const API_TYPES = resolve(__dirname, "./api-types.ts");

/** The string each `SnapshotStatus` variant serialises to, from `as_str`. */
function rustStatuses(): string[] {
  const source = readFileSync(MODEL, "utf8");
  const start = source.indexOf("impl SnapshotStatus {");
  const asStr = source.slice(start, source.indexOf("pub fn parse", start));
  return [...asStr.matchAll(/=> "([a-z_]+)"/g)].map((match) => match[1]);
}

function tsStatuses(): string[] {
  const source = readFileSync(API_TYPES, "utf8");
  const start = source.indexOf("export type SnapshotStatus =");
  const union = source.slice(start, source.indexOf(";", start));
  return [...union.matchAll(/"([a-z_]+)"/g)].map((match) => match[1]);
}

describe("SnapshotStatus mirror", () => {
  it("names every status the Rust model can store", () => {
    expect(tsStatuses().sort()).toEqual(rustStatuses().sort());
  });
});
