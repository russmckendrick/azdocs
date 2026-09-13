import { describe, expect, it } from "vitest";
import {
  effectiveTenant,
  newTenant,
  removeTenant,
  renameTenant,
  splitSettingList,
} from "./settings-model";
import type { SettingsValues } from "./types";

function values(): SettingsValues {
  return {
    schema_version: 2,
    default_tenant: "acme",
    tenants: { acme: newTenant("Acme") },
    collect: { subscriptions: ["sub"], concurrency: 4 },
    audit: { required_tags: ["owner"] },
    storage: { db_path: "estate.db" },
    branding: {
      company: "Shared",
      title: "Report",
      subtitle: "",
      primary_color: "#112233",
      accent_color: "#445566",
      logo: null,
      page_size: "a4",
      margin: "2cm",
      footer: "",
      theme: "field-report",
      labels: "en",
      font_family: "",
      mono_family: "",
      font_dir: null,
    },
  };
}
describe("settings", () => {
  it("inherits nullable overrides from generated DTOs", () => {
    const v = values();
    v.tenants.acme.collect.subscriptions = null;
    v.tenants.acme.branding.company = null;
    expect(effectiveTenant(v, "acme").collect.subscriptions).toEqual(["sub"]);
    expect(effectiveTenant(v, "acme").branding.company).toBe("Shared");
  });
  it("preserves explicit empty lists instead of inheriting them", () => {
    const v = values();
    v.tenants.acme.collect.subscriptions = [];
    v.tenants.acme.audit.required_tags = [];
    expect(effectiveTenant(v, "acme").collect.subscriptions).toEqual([]);
    expect(effectiveTenant(v, "acme").audit.required_tags).toEqual([]);
  });
  it("renames the reference and default together without changing identity", () => {
    const v = values();
    v.tenants.acme.tenant_id = "identity";
    const renamed = renameTenant(v, "acme", "customer");
    expect(renamed.default_tenant).toBe("customer");
    expect(renamed.tenants.customer.tenant_id).toBe("identity");
    expect(v.tenants.acme).toBeDefined();
  });
  it("rejects reference collisions", () => {
    const v = values();
    v.tenants.other = newTenant("Other");
    expect(renameTenant(v, "acme", "other")).toBe(v);
  });
  it("removes only the profile and clears its default", () => {
    const v = values();
    expect(removeTenant(v, "acme").default_tenant).toBeNull();
    expect(v.tenants.acme).toBeDefined();
  });
  it("normalizes comma and line separated entries without empty members", () => {
    expect(splitSettingList("owner,\n environment,owner, ")).toEqual([
      "owner",
      "environment",
    ]);
  });
});
