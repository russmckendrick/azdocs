import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { mockBootstrap } from "../mock-data";
import { resourceHistoryScale } from "./dashboard-model";
import { HistoryChart } from "./OverviewView";

describe("resource history axis labels", () => {
  it("renders distinct complete integer labels for nearby totals on large estates", () => {
    for (const counts of [[14, 15], [4000, 4001], [1000000, 1000001]]) {
      const series = counts.map((resources, index) => ({ ...mockBootstrap.snapshots[1 - index], status: "complete" as const, resources }));
      const markup = renderToStaticMarkup(createElement(HistoryChart, { series, onSelect: () => {} }));
      for (const tick of resourceHistoryScale(counts).ticks) {
        expect(markup).toContain(`>${tick.toLocaleString()}</text>`);
      }
    }
  });
});
