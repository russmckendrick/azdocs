import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { FindingsView } from "./FindingsView";
import { DEFAULT_LABELS } from "../labels";
import * as api from "../api";
import { mockEstate } from "../mock-data";

vi.mock("../api", async () => (await import("../api-mock")).apiMock());

const words = DEFAULT_LABELS.desktop.findings;
const common = DEFAULT_LABELS.common;

function rows() {
  return within(screen.getByRole("listbox", { name: words.list_aria })).queryAllByRole("option");
}

describe("FindingsView", () => {
  it("filters by severity box, search and an exact resource selection", () => {
    const { rerender } = render(<FindingsView estate={mockEstate} search="" onOpenResource={vi.fn()} />);
    expect(rows()).toHaveLength(mockEstate.findings.length);

    const tally = within(screen.getByLabelText(words.severity_counts_aria));
    fireEvent.click(tally.getByRole("button", { name: new RegExp(common.severity.high.name) }));
    expect(rows()).toHaveLength(mockEstate.findings.filter((f) => f.severity === "high").length);
    fireEvent.click(tally.getByRole("button", { name: new RegExp(common.severity.high.name) }));

    const needle = mockEstate.findings[0].title.split(" ")[0];
    rerender(<FindingsView estate={mockEstate} search={needle} onOpenResource={vi.fn()} />);
    expect(rows().length).toBeGreaterThan(0);
    expect(rows().length).toBeLessThan(mockEstate.findings.length);
    expect(rows().every((row) => row.textContent?.toLowerCase().includes(needle.toLowerCase()))).toBe(true);

    const target = mockEstate.findings.find((f) => f.resourceId)!;
    rerender(
      <FindingsView estate={mockEstate} search="" onOpenResource={vi.fn()} dashboardFilter={{ resourceIds: [target.resourceId!] }} />,
    );
    expect(rows()).toHaveLength(mockEstate.findings.filter((f) => f.resourceId === target.resourceId).length);
  });

  it("opens the evidence drawer for a finding and links to its resource", () => {
    const onOpenResource = vi.fn();
    render(<FindingsView estate={mockEstate} search="" onOpenResource={onOpenResource} />);
    const first = mockEstate.findings.find((f) => f.resourceId)!;

    fireEvent.click(screen.getByRole("button", { name: new RegExp(first.title) }));

    expect(screen.getByText(words.stored_evidence)).toBeInTheDocument();
    fireEvent.click(screen.getByText(words.affected_resource).closest("button")!);
    expect(onOpenResource).toHaveBeenCalledWith(first.resourceId);
  });

  it("copies the resource ids of ticked findings and saves them as csv", async () => {
    render(<FindingsView estate={mockEstate} search="" onOpenResource={vi.fn()} />);
    const copy = screen.getByRole("button", { name: new RegExp(words.copy_ids) });
    expect(copy).toBeDisabled();

    fireEvent.click(screen.getByRole("checkbox", { name: words.select_all }));
    expect(screen.getByRole("status")).toHaveTextContent(String(mockEstate.findings.length));

    fireEvent.click(copy);
    await waitFor(() => expect(api.copyText).toHaveBeenCalled());
    const ids = new Set(mockEstate.findings.flatMap((f) => (f.resourceId ? [f.resourceId] : [])));
    expect(vi.mocked(api.copyText).mock.calls[0][0].split("\n")).toHaveLength(ids.size);

    fireEvent.click(screen.getByRole("button", { name: new RegExp(words.export_csv) }));
    await waitFor(() => expect(api.saveTextFile).toHaveBeenCalled());
    const [name, csv] = vi.mocked(api.saveTextFile).mock.calls[0];
    expect(name).toBe(words.csv_name);
    expect(csv.split("\n")[0]).toContain(common.columns.severity);
  });
});
