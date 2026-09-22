import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DEFAULT_LABELS } from "../labels";
import { mockBootstrap } from "../mock-data";
import { fill, plural } from "../format";
import { HistoryChart } from "./HistoryChart";
import { resourceHistoryMarkers } from "./dashboard-model";

const words = DEFAULT_LABELS.desktop.overview;
const series = [
  {
    ...mockBootstrap.snapshots[0],
    id: "start",
    status: "complete" as const,
    resources: 294,
    createdAt: "2026-08-16T08:00:00Z",
  },
  ...Array.from({ length: 5 }, (_, index) => ({
    ...mockBootstrap.snapshots[0],
    id: `close-${index}`,
    status: "complete" as const,
    resources: 307,
    createdAt: `2026-09-22T08:0${index}:00Z`,
  })),
  {
    ...mockBootstrap.snapshots[0],
    id: "failed",
    status: "failed" as const,
    resources: 0,
    createdAt: "2026-09-22T08:05:00Z",
  },
  {
    ...mockBootstrap.snapshots[0],
    id: "latest",
    status: "warnings" as const,
    resources: 307,
    createdAt: "2026-09-22T08:06:00Z",
  },
];

describe("resource history readability", () => {
  it("separates incomplete collections from counts and keeps clustered snapshots selectable", () => {
    const onSelect = vi.fn();
    const { container } = render(
      <HistoryChart series={series} onSelect={onSelect} />,
    );
    const plot = screen.getByRole("group", { name: words.growth_aria });
    expect(
      within(plot).queryByRole("button", { name: /failed/ }),
    ).not.toBeInTheDocument();
    expect(within(plot).getAllByRole("button")).toHaveLength(2);
    // Connect valid inventories across failed attempts without inventing a resource count.
    expect(
      container
        .querySelector(".dashboard-history-line")
        ?.getAttribute("d")
        ?.match(/M/g),
    ).toHaveLength(1);
    expect(
      screen.getByText(fill(words.dashboard.history_incomplete, { count: 1 })),
    ).toBeVisible();
    fireEvent.click(
      screen.getByText(
        plural(words.dashboard.history_snapshots, series.length),
      ),
    );
    const records = within(screen.getByRole("list"));
    expect(records.getAllByRole("button")).toHaveLength(series.length);
    fireEvent.click(records.getByRole("button", { name: /failed/ }));
    expect(onSelect).toHaveBeenLastCalledWith(series[6]);
    fireEvent.click(records.getAllByRole("button")[3]);
    expect(onSelect).toHaveBeenLastCalledWith(series[4]);
  });

  it("avoids an invented resource axis when every collection is incomplete", () => {
    render(<HistoryChart series={[series[6]]} onSelect={vi.fn()} />);
    expect(
      screen.queryByRole("group", { name: words.growth_aria }),
    ).not.toBeInTheDocument();
    expect(screen.getByText(words.dashboard.history_empty)).toBeVisible();
  });

  it("keeps marker hit targets apart while favouring the latest nearby observation", () => {
    const points = Array.from({ length: 200 }, (_, index) => ({
      x: index,
      y: 20,
    }));
    const markers = resourceHistoryMarkers(points);
    expect(markers.at(-1)).toBe(points.at(-1));
    for (let i = 1; i < markers.length; i++) {
      expect(
        Math.hypot(
          markers[i].x - markers[i - 1].x,
          markers[i].y - markers[i - 1].y,
        ),
      ).toBeGreaterThanOrEqual(24);
    }
    expect(points).toHaveLength(200);
  });
});
