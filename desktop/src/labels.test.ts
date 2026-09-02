import { describe, expect, it } from "vitest";
import { DEFAULT_LABELS } from "./labels";
import { fill, plural } from "./format";
import { mockBootstrap } from "./mock-data";
import { EXPORT_PRESETS } from "./components/export-presets";

describe("fill", () => {
  it("unit_substitutes_named_placeholders_when_given_values", () => {
    expect(fill("{count} of {total}", { count: 3, total: 9 })).toBe("3 of 9");
  });

  it("unit_leaves_unknown_placeholders_visible_when_a_value_is_missing", () => {
    expect(fill("{count} of {total}", { count: 3 })).toBe("3 of {total}");
  });

  it("unit_does_not_rescan_substituted_values_when_they_contain_braces", () => {
    expect(fill("id {id}", { id: "{name}" })).toBe("id {name}");
  });
});

describe("plural", () => {
  const forms = { one: "{count} resource", other: "{count} resources" };

  it("unit_picks_the_singular_form_when_the_count_is_exactly_one", () => {
    expect(plural(forms, 1)).toBe("1 resource");
  });

  it("unit_picks_the_plural_form_when_the_count_is_not_one", () => {
    expect(plural(forms, 0)).toBe("0 resources");
    expect(plural(forms, 2)).toBe("2 resources");
  });
});

describe("DEFAULT_LABELS", () => {
  function walk(value: unknown, path: string, visit: (path: string, text: string) => void) {
    if (typeof value === "string") {
      visit(path, value);
      return;
    }
    if (value && typeof value === "object") {
      for (const [key, inner] of Object.entries(value)) walk(inner, `${path}.${key}`, visit);
      return;
    }
    throw new Error(`${path}: labels hold strings and tables, found ${String(value)}`);
  }

  it("unit_has_no_empty_leaves_when_generated_from_the_built_in_file", () => {
    walk(DEFAULT_LABELS, "labels", (path, text) => {
      expect(text, path).not.toBe("");
    });
  });

  it("unit_names_every_export_preset_when_the_view_lists_one", () => {
    for (const preset of EXPORT_PRESETS) {
      expect(DEFAULT_LABELS.desktop.exports.presets[preset.id].label).toBeTruthy();
    }
  });

  it("unit_gives_the_browser_preview_the_built_ins_when_it_bootstraps", () => {
    expect(mockBootstrap.labels).toBe(DEFAULT_LABELS);
  });
});
