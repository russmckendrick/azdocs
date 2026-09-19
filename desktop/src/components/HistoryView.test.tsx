import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { HistoryView } from "./HistoryView";
import { DEFAULT_LABELS } from "../labels";
import * as api from "../api";
import { mockBootstrap, mockComparison, mockEstate } from "../mock-data";

vi.mock("../api", async () => (await import("../api-mock")).apiMock());

const words = DEFAULT_LABELS.desktop.history;

function renderHistory() {
  render(
    <HistoryView
      bootstrap={mockBootstrap}
      estate={mockEstate}
      previousComparison={mockComparison}
      onLoadSnapshot={vi.fn()}
      onOpenResource={vi.fn()}
    />,
  );
}

describe("HistoryView", () => {
  it("counts added, changed and removed resources and lists the changed fields", () => {
    renderHistory();

    expect(screen.getByText(words.added).previousSibling).toHaveTextContent(String(mockComparison.added.length));
    expect(screen.getByText(words.removed).previousSibling).toHaveTextContent(String(mockComparison.removed.length));
    const [changedId, fields] = Object.entries(mockComparison.fields)[0];
    const changed = mockEstate.resources.find((r) => r.id === changedId)!;
    const summary = screen.getByText(changed.name).closest("summary")!;
    fireEvent.click(summary);
    for (const change of fields) {
      expect(screen.getByText(`${change.field}.${change.path}`)).toBeInTheDocument();
    }
  });

  it("lists new and resolved findings and the trend", () => {
    renderHistory();

    expect(screen.getByText(words.new_findings)).toBeInTheDocument();
    expect(screen.getByText(mockComparison.findingsAdded[0].title)).toBeInTheDocument();
    expect(screen.getByText(words.resolved_findings)).toBeInTheDocument();
    expect(screen.getByText(words.trend)).toBeInTheDocument();
    expect(screen.getAllByRole("row")).toHaveLength(mockEstate.trend.length + 1 + Object.values(mockComparison.fields).flat().length + Object.keys(mockComparison.fields).length);
  });

  it("keeps the last good comparison and says why a new one failed", async () => {
    vi.mocked(api.compareSnapshots).mockRejectedValue(new Error("database is locked"));
    renderHistory();
    const older = mockBootstrap.snapshots.find((s) => s.id !== mockEstate.id && s.id !== mockComparison.baseSnapshotId)!;

    fireEvent.change(screen.getByRole("combobox", { name: words.base_aria }), { target: { value: older.id } });

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("database is locked");
    expect(screen.getByText(words.added).previousSibling).toHaveTextContent(String(mockComparison.added.length));
    fireEvent.click(await screen.findByRole("button", { name: DEFAULT_LABELS.desktop.shell.dismiss }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });
});
