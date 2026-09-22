import { describe, expect, it } from "vitest";
import { mockEstate, mockQueryPack, mockQueryRows } from "../mock-data";
import { detailColumns, joinRows, typeQueries } from "./resource-queries";

const VNET = "microsoft.network/virtualnetworks";
const vnets = mockEstate.resources.filter((resource) => resource.azureType === VNET);

describe("typeQueries", () => {
  it("puts the pack's default table first, then related queries by name", () => {
    const names = typeQueries(mockQueryPack, mockEstate.queryRuns, VNET).map((def) => def.name);

    expect(names).toEqual(["virtual_networks", "subnets"]);
  });

  it("offers no query that stored no rows for the snapshot", () => {
    const runs = mockEstate.queryRuns.map((run) =>
      run.queryName === "subnets" ? { ...run, rowCount: 0 } : run,
    );

    expect(typeQueries(mockQueryPack, runs, VNET).map((def) => def.name)).toEqual(["virtual_networks"]);
  });
});

describe("detailColumns", () => {
  it("drops the row's own identity when the row is the resource", () => {
    expect(detailColumns(mockQueryRows("virtual_networks").columns, "id")).toEqual([
      "addressPrefixes",
      "dnsServers",
      "subnetCount",
    ]);
  });

  it("keeps a child row's name but drops the column it joined on", () => {
    const columns = detailColumns(mockQueryRows("subnets").columns, "vnetId");

    expect(columns).toContain("name");
    expect(columns).not.toContain("vnetId");
  });
});

describe("joinRows", () => {
  it("places every child row under its parent, ignoring id casing", () => {
    const rows = mockQueryRows("subnets").rows.map((row) => ({
      ...row,
      vnetId: String(row.vnetId).toUpperCase(),
    }));

    const { rows: joined, missing } = joinRows(vnets, rows, "vnetId");

    expect(missing).toBe(0);
    expect(joined).toHaveLength(3);
    expect(joined.filter((line) => line.first)).toHaveLength(vnets.length);
  });

  it("keeps a resource without a row as one trailing empty line", () => {
    const rows = mockQueryRows("virtual_networks").rows.slice(0, 1);

    const { rows: joined, missing } = joinRows(vnets, rows, "id");

    expect(missing).toBe(vnets.length - 1);
    expect(joined).toHaveLength(vnets.length);
    expect(joined.at(-1)?.row).toBeUndefined();
  });

  it("leaves out rows for resources outside the view", () => {
    const { rows: joined } = joinRows(vnets.slice(0, 1), mockQueryRows("virtual_networks").rows, "id");

    expect(joined).toHaveLength(1);
  });
});
