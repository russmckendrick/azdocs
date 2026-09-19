import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { GovernanceView } from "./GovernanceView";
import { DEFAULT_LABELS } from "../labels";
import { fill } from "../format";
import { mockEstate } from "../mock-data";

vi.mock("../api", async () => (await import("../api-mock")).apiMock());

const common = DEFAULT_LABELS.common;

describe("GovernanceView", () => {
  it("says when the key and group summaries were cut short", () => {
    const estate = {
      ...mockEstate,
      governance: { ...mockEstate.governance, topKeysTotal: 40, worstGroupsTotal: 9 },
    };
    render(<GovernanceView estate={estate} requiredTags={["env"]} onOpenFindings={vi.fn()} />);

    expect(
      screen.getByText(fill(common.governance.top_keys_note, { shown: estate.governance.topKeys.length, total: 40 })),
    ).toBeInTheDocument();
    expect(
      screen.getByText(fill(common.governance.worst_groups_note, { shown: estate.governance.worstGroups.length, total: 9 })),
    ).toBeInTheDocument();
  });

  it("opens a group's governance findings through the result chip", () => {
    const onOpenFindings = vi.fn();
    render(<GovernanceView estate={mockEstate} requiredTags={["env"]} onOpenFindings={onOpenFindings} />);
    const group = mockEstate.governance.worstGroups[0];

    fireEvent.click(screen.getByRole("button", { name: group.name }));

    expect(onOpenFindings).toHaveBeenCalledWith(
      expect.objectContaining({
        view: "findings",
        filter: expect.objectContaining({ category: "governance", resourceGroup: group.name.toLowerCase() }),
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: new RegExp(DEFAULT_LABELS.desktop.governance.governance_findings) }));
    expect(onOpenFindings).toHaveBeenLastCalledWith();
  });
});
