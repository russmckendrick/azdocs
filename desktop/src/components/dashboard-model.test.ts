import { describe, expect, it } from "vitest";
import { mockBootstrap, mockComparison, mockEstate } from "../mock-data";
import {
  dashboardComparison,
  dashboardData,
  dashboardHistory,
  findingMatchesDashboard,
  queryCoverage,
  resourceMatchesDashboard,
  resourceHistoryScale,
} from "./dashboard-model";

describe("dashboard evidence", () => {
  it("conserves scoped resources across type and region charts", () => {
    for (const subscription of mockEstate.subscriptions) {
      const data = dashboardData(mockEstate, subscription.id);
      expect(data.types.reduce((sum, type) => sum + type.count, 0)).toBe(
        data.resources.length,
      );
      expect(
        data.locations.reduce((sum, location) => sum + location.count, 0),
      ).toBe(data.resources.length);
      expect(
        data.resources.every(
          (resource) => resource.subscriptionId === subscription.id,
        ),
      ).toBe(true);
      expect(data.tagged.length).toBeLessThanOrEqual(data.resources.length);
    }
  });
  it("uses the Rust-provided coverage percentages without rounding again", () => {
    expect(dashboardData(mockEstate).percent).toBe(
      mockEstate.tagCoverage.percent,
    );
    for (const subscription of mockEstate.governance.subscriptions)
      expect(
        dashboardData(mockEstate, subscription.subscriptionId).percent,
      ).toBe(subscription.percent);
  });
  it("does not assign an estate-level finding to a subscription", () => {
    const estate = {
      ...mockEstate,
      findings: [
        ...mockEstate.findings,
        { ...mockEstate.findings[0], resourceId: undefined },
      ],
    };
    expect(dashboardData(estate).findings).toHaveLength(estate.findings.length);
    expect(
      dashboardData(estate, estate.subscriptions[0].id).findings.every(
        (finding) => finding.resourceId,
      ),
    ).toBe(true);
  });
  it("keeps cross-subscription edges when either endpoint is in scope", () => {
    const source = mockEstate.resources[0];
    const target = mockEstate.resources.find(
      (resource) => resource.subscriptionId !== source.subscriptionId,
    )!;
    const estate = {
      ...mockEstate,
      edges: [
        {
          sourceId: source.id,
          targetId: target.id,
          kind: mockEstate.edges[0].kind,
        },
      ],
    };
    expect(dashboardData(estate, source.subscriptionId).edges).toHaveLength(1);
  });
  it("treats an empty exact resource selection as no results", () => {
    expect(
      resourceMatchesDashboard(mockEstate.resources[0], { resourceIds: [] }),
    ).toBe(false);
    expect(
      findingMatchesDashboard(mockEstate.findings[0], mockEstate, {
        resourceIds: [],
      }),
    ).toBe(false);
  });
  it("narrows resources and findings to one resource group", () => {
    const resource = mockEstate.resources.find((item) => item.findingCount > 0 && item.resourceGroup)!;
    const finding = mockEstate.findings.find((item) => item.resourceId === resource.id)!;
    const filter = { subscriptionId: resource.subscriptionId, resourceGroup: resource.resourceGroup! };
    expect(resourceMatchesDashboard(resource, filter)).toBe(true);
    expect(resourceMatchesDashboard({ ...resource, resourceGroup: "elsewhere" }, filter)).toBe(false);
    expect(findingMatchesDashboard(finding, mockEstate, filter)).toBe(true);
    expect(findingMatchesDashboard({ ...finding, resourceId: null }, mockEstate, filter)).toBe(false);
  });
  it("distinguishes a missing location from an unfiltered location", () => {
    const resource = { ...mockEstate.resources[0], location: null };
    expect(resourceMatchesDashboard(resource, { location: "" })).toBe(true);
    expect(
      resourceMatchesDashboard(
        mockEstate.resources.find((item) => item.location)!,
        { location: "" },
      ),
    ).toBe(false);
  });
  it("matches query names exactly rather than as search substrings", () => {
    const finding = mockEstate.findings[0];
    expect(
      findingMatchesDashboard(finding, mockEstate, {
        queryName: finding.queryName,
      }),
    ).toBe(true);
    expect(
      findingMatchesDashboard(finding, mockEstate, {
        queryName: finding.queryName.slice(0, -1),
      }),
    ).toBe(false);
  });
  it("excludes future and foreign-tenant history while retaining incomplete records", () => {
    const bootstrap = {
      ...mockBootstrap,
      snapshots: [
        ...mockBootstrap.snapshots,
        { ...mockBootstrap.snapshots[0], id: "foreign", tenantId: "another" },
        {
          ...mockBootstrap.snapshots[0],
          id: "future",
          createdAt: "2099-01-01T00:00:00Z",
        },
      ],
    };
    const history = dashboardHistory(bootstrap, mockEstate, 0);
    expect(
      history.some(
        (snapshot) => snapshot.id === "foreign" || snapshot.id === "future",
      ),
    ).toBe(false);
    expect(history.some((snapshot) => snapshot.status === "partial")).toBe(
      true,
    );
    expect(history.map((snapshot) => snapshot.createdAt)).toEqual(
      history.map((snapshot) => snapshot.createdAt).sort(),
    );
  });
  it("does not count missing query outcomes as successful checks", () => {
    const estate = {
      ...mockEstate,
      queryRuns: [
        { queryName: "a", category: "test", rowCount: 0 },
        { queryName: "b", category: "test", error: "failed" },
        { queryName: "c", category: "test" },
      ],
    };
    expect(queryCoverage(estate)[0]).toMatchObject({ succeeded: 1, total: 3 });
  });
  it("keeps inventory comparisons available when only audit checks have warnings", () => {
    expect(dashboardComparison(mockBootstrap, { ...mockEstate, status: "warnings" }, mockComparison)?.diff).toBe(mockComparison);
  });
  it("suppresses comparisons involving incomplete or foreign-tenant snapshots", () => {
    expect(
      dashboardComparison(
        mockBootstrap,
        { ...mockEstate, status: "partial" },
        mockComparison,
      ),
    ).toBeUndefined();
    const bootstrap = {
      ...mockBootstrap,
      snapshots: mockBootstrap.snapshots.map((snapshot) => ({
        ...snapshot,
        tenantId: "another",
      })),
    };
    expect(
      dashboardComparison(bootstrap, mockEstate, mockComparison),
    ).toBeUndefined();
    expect(
      dashboardComparison(mockBootstrap, mockEstate, mockComparison)?.diff,
    ).toBe(mockComparison);
    expect(
      dashboardComparison(mockBootstrap, mockEstate, undefined),
    ).toBeUndefined();
  });
});

describe("resource history scale", () => {
  it("excludes running snapshots entirely from the history series", () => {
    const bootstrap = { ...mockBootstrap, snapshots: [...mockBootstrap.snapshots, { ...mockBootstrap.snapshots[0], id: "in-progress", status: "running" as const, resources: 0 }] };
    expect(dashboardHistory(bootstrap, mockEstate, 0).some(snapshot => snapshot.status === "running")).toBe(false);
  });
  it("zooms around small changes instead of forcing the axis to zero", () => {
    const scale = resourceHistoryScale([303, 305, 307]);
    expect(scale.min).toBeGreaterThan(290);
    expect(scale.min).toBeLessThan(303);
    expect(scale.max).toBeGreaterThan(307);
    expect(scale.max - scale.min).toBeLessThanOrEqual(10);
  });
  it("has a nonzero range with integer ticks for empty, constant and zero histories", () => {
    for (const values of [[], [0], [307], [307, 307], [0, 4000]]) {
      const scale = resourceHistoryScale(values);
      expect(scale.min).toBeGreaterThanOrEqual(0);
      expect(scale.max).toBeGreaterThan(scale.min);
      expect(scale.ticks.every(Number.isInteger)).toBe(true);
      expect(scale.ticks.length).toBeLessThanOrEqual(7);
      for (const value of values) expect(value >= scale.min && value <= scale.max).toBe(true);
    }
  });
});
