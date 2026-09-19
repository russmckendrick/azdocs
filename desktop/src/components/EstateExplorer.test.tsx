import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { EstateExplorer } from "./EstateExplorer";
import { DEFAULT_LABELS } from "../labels";
import { mockEstate } from "../mock-data";

vi.mock("../api", async () => (await import("../api-mock")).apiMock());

const words = DEFAULT_LABELS.desktop.estate;

function renderExplorer(search = "") {
  const onSelectResource = vi.fn();
  render(
    <EstateExplorer estate={mockEstate} search={search} scope={{}} onScopeChange={vi.fn()} onSelectResource={onSelectResource} />,
  );
  return { onSelectResource };
}

function visibleRows() {
  return within(screen.getByRole("listbox", { name: words.list_aria })).getAllByRole("option");
}

describe("EstateExplorer", () => {
  it("lists every resource and narrows by type", () => {
    renderExplorer();
    expect(visibleRows()).toHaveLength(mockEstate.resources.length);

    fireEvent.change(screen.getByRole("combobox", { name: words.type_filter_aria }), {
      target: { value: "microsoft.network/virtualnetworks" },
    });

    const vnets = mockEstate.resources.filter((r) => r.azureType === "microsoft.network/virtualnetworks");
    expect(visibleRows()).toHaveLength(vnets.length);
  });

  it("offers the estate's tag keys as a facet and remembers the choice per tenant", () => {
    renderExplorer();
    const facet = screen.getByRole("combobox", { name: words.tag_filter_aria });
    const keys = within(facet).getAllByRole("option").map((option) => option.textContent);
    expect(keys).toEqual([words.all_tags, "env", "owner"]);

    fireEvent.change(facet, { target: { value: "env" } });

    const tagged = mockEstate.resources.filter((r) => r.tags && "env" in r.tags);
    expect(visibleRows()).toHaveLength(tagged.length);
    const stored = JSON.parse(window.localStorage.getItem(`azdocs-explorer-filters:${mockEstate.tenantId}`)!);
    expect(stored.tagKey).toBe("env");
    expect(screen.getByRole("combobox", { name: words.tag_value_aria })).toBeInTheDocument();
  });

  it("restores a remembered filter and marks the active row", () => {
    window.localStorage.setItem(
      `azdocs-explorer-filters:${mockEstate.tenantId}`,
      JSON.stringify({ locationFilter: "ukwest" }),
    );
    const { onSelectResource } = renderExplorer();

    const rows = visibleRows();
    expect(rows).toHaveLength(mockEstate.resources.filter((r) => r.location === "ukwest").length);
    fireEvent.click(rows[0]);

    expect(onSelectResource).toHaveBeenCalledWith(rows[0].dataset.resourceId);
    expect(rows[0]).toHaveAttribute("aria-selected", "true");
  });

  it("searches names, types and tags through the shared matcher", () => {
    renderExplorer("platform");
    expect(visibleRows().length).toBeGreaterThan(0);
    expect(visibleRows().every((row) => row.dataset.resourceId?.includes("sub-prod"))).toBe(true);
  });
});
