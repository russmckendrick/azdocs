import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { displayKind, displayLocation, humanizeIdentifier } from "./azure-values";
import type { AzureMetadata } from "./types";

/**
 * `humanizeIdentifier` is a hand port of `humanize_identifier` in
 * src/model/azure_values.rs, and since the CLI reports started printing
 * friendly kinds, *both* implementations are live — the Rust one renders the
 * PDF/DOCX/Markdown, this one renders the desktop. The same snapshot must read
 * the same way in either.
 *
 * These cases are asserted identically in the Rust module's own tests
 * (`unit_collapses_whitespace_around_a_comma_when_humanising` and friends), so
 * a change on one side without the other fails on that side.
 */
const SHARED_CASES: Array<[input: string, expected: string]> = [
  ["GlobalDocumentDB", "Global Document DB"],
  ["app,linux", "app, linux"],
  ["app , linux", "app, linux"],
  ["BlobStorage", "Blob Storage"],
  ["StorageV2", "Storage V2"],
  ["functionapp,linux,container", "functionapp, linux, container"],
  ["MongoDB", "Mongo DB"],
  ["user_assigned-identity", "user assigned identity"],
  ["V2", "V2"],
  ["XMLHttpRequest", "XML Http Request"],
  ["a,b", "a, b"],
  ["  padded  ", "padded"],
  ["AAABbb", "AAA Bbb"],
  ["x1Y", "x1 Y"],
];

const metadata: AzureMetadata = {
  locations: { uksouth: "UK South" },
  regions: {},
  kinds: {
    "microsoft.documentdb/databaseaccounts:globaldocumentdb": "Global Document DB",
    "*:storagev2": "Storage V2",
  },
};

describe("humanizeIdentifier", () => {
  it.each(SHARED_CASES)(
    "unit_matches_the_rust_humaniser_when_given_%j",
    (input, expected) => {
      expect(humanizeIdentifier(input)).toBe(expected);
    },
  );

  it("unit_asserts_the_same_cases_the_rust_tests_do", () => {
    // Guard the guard: if the Rust tests stop covering these, this pairing is
    // no longer protecting anything and should be revisited.
    const rust = readFileSync(
      fileURLToPath(new URL("../../src/model/azure_values.rs", import.meta.url)),
      "utf8",
    );
    for (const probe of ["app , linux", "  padded  ", "a,b"]) {
      expect(rust, `src/model/azure_values.rs should assert ${probe}`).toContain(
        JSON.stringify(probe).slice(1, -1),
      );
    }
  });
});

describe("displayLocation", () => {
  it("unit_maps_a_known_location_when_resolving", () => {
    expect(displayLocation(metadata, "UKSOUTH")).toBe("UK South");
  });

  it("unit_passes_an_unknown_location_through_when_resolving", () => {
    expect(displayLocation(metadata, "customedgezone")).toBe("customedgezone");
  });

  it("unit_returns_the_missing_label_when_there_is_no_location", () => {
    expect(displayLocation(metadata, null)).toBe("Global");
    expect(displayLocation(metadata, undefined, "Not stored")).toBe("Not stored");
  });
});

describe("displayKind", () => {
  it("unit_prefers_the_type_scoped_mapping_when_one_exists", () => {
    expect(
      displayKind(metadata, "microsoft.documentdb/databaseaccounts", "GlobalDocumentDB"),
    ).toBe("Global Document DB");
  });

  it("unit_falls_back_to_the_wildcard_mapping_when_the_type_is_unmapped", () => {
    expect(displayKind(metadata, "microsoft.custom/widgets", "StorageV2")).toBe("Storage V2");
  });

  it("unit_humanises_when_no_mapping_matches", () => {
    expect(displayKind(metadata, "microsoft.custom/widgets", "SQLDatabaseV2")).toBe(
      "SQL Database V2",
    );
  });
});
