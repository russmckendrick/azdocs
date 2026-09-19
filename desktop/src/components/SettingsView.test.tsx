import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SettingsView } from "./SettingsView";
import { DEFAULT_LABELS } from "../labels";
import { fill } from "../format";
import * as api from "../api";
import { mockBootstrap, mockEstate } from "../mock-data";

vi.mock("../api", async () => (await import("../api-mock")).apiMock());

const settings = DEFAULT_LABELS.desktop.settings;

function renderSettings(overrides: Partial<React.ComponentProps<typeof SettingsView>> = {}) {
  const props = {
    bootstrap: mockBootstrap,
    estate: mockEstate,
    themePreference: "system" as const,
    resolvedTheme: "light" as const,
    onThemeChange: vi.fn(),
    onOpenDatabase: vi.fn(),
    onConfigChange: vi.fn(async () => {}),
    onDirtyChange: vi.fn(),
    onShowShortcuts: vi.fn(),
    blocked: false,
    ...overrides,
  };
  render(<SettingsView {...props} />);
  return props;
}

async function openApplicationTab() {
  await waitFor(() => expect(api.getSettings).toHaveBeenCalled());
  fireEvent.click(await screen.findByRole("button", { name: settings.editor.application }));
}

describe("SettingsView", () => {
  it("names the running version and opens the docs and shortcuts from About", async () => {
    const props = renderSettings();
    await openApplicationTab();

    expect(
      await screen.findByText(fill(settings.about_detail, { version: mockBootstrap.appVersion })),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: new RegExp(settings.shortcuts) }));
    expect(props.onShowShortcuts).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: new RegExp(settings.docs) }));
    await waitFor(() => expect(api.openDocs).toHaveBeenCalled());
  });

  it("reports a dirty draft to the shell and clears it on discard", async () => {
    const props = renderSettings();
    await openApplicationTab();
    const path = await screen.findByLabelText(settings.editor.field_db_path);

    fireEvent.change(path, { target: { value: "/tmp/other.db" } });

    await waitFor(() => expect(props.onDirtyChange).toHaveBeenLastCalledWith(true));
    fireEvent.click(screen.getByRole("button", { name: settings.editor.discard }));
    await waitFor(() => expect(props.onDirtyChange).toHaveBeenLastCalledWith(false));
  });
});
