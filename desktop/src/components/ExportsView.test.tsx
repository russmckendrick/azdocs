import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import * as api from "../api";
import { ExportsView } from "./ExportsView";
import { DEFAULT_LABELS } from "../labels";
import { mockEstate } from "../mock-data";
import reportThemes from "../generated-report-themes.json";

vi.mock("../api", async () => (await import("../api-mock")).apiMock());

const words = DEFAULT_LABELS.desktop.exports;

describe("ExportsView", () => {
  it("opens the style picker on the configured theme and marks it", async () => {
    render(<ExportsView estate={mockEstate} />);

    const configured = reportThemes.themes.find(
      (theme) => theme.name === reportThemes.configured,
    )!;
    const option = await screen.findByRole("radio", { name: new RegExp(configured.title) });
    expect(option).toBeChecked();
    expect(screen.getByText(words.theme_configured)).toBeInTheDocument();
  });

  it("sends the chosen style with a report export", async () => {
    render(<ExportsView estate={mockEstate} />);
    const other = reportThemes.themes.find(
      (theme) => theme.name !== reportThemes.configured,
    )!;

    fireEvent.click(await screen.findByRole("radio", { name: new RegExp(other.title) }));
    fireEvent.click(screen.getByRole("button", { name: /Preview|Export/ }));

    await waitFor(() => expect(api.exportSnapshot).toHaveBeenCalled());
    expect(vi.mocked(api.exportSnapshot).mock.calls[0]![0]).toMatchObject({
      formats: ["pdf"],
      theme: other.name,
    });
  });

  it("hides the style picker for deliverables that are not documents", async () => {
    render(<ExportsView estate={mockEstate} />);
    await screen.findByText(words.theme_title);

    fireEvent.click(
      screen.getByRole("radio", { name: new RegExp(words.presets["data-workbook"].label) }),
    );

    expect(screen.queryByText(words.theme_title)).not.toBeInTheDocument();
  });
});
