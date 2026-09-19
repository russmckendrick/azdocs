import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import App from "./App";
import { DEFAULT_LABELS } from "./labels";
import * as api from "./api";
import { mockBootstrap } from "./mock-data";

vi.mock("./api", async () => (await import("./api-mock")).apiMock());

const words = DEFAULT_LABELS.desktop;

describe("App shell", () => {
  beforeEach(() => {
    delete document.documentElement.dataset.theme;
  });

  it("shows the empty workspace when no snapshot is stored", async () => {
    vi.mocked(api.getBootstrap).mockResolvedValueOnce({ ...mockBootstrap, snapshots: [], latestSnapshotId: null });

    render(<App />);

    expect(await screen.findByRole("heading", { name: words.shell.empty_title })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: words.shell.open_database })).toBeInTheDocument();
    expect(api.getSnapshot).not.toHaveBeenCalled();
  });

  it("opens the latest snapshot, lists the navigation and names the version", async () => {
    render(<App />);

    expect(await screen.findByRole("button", { name: words.nav.estate })).toBeInTheDocument();
    expect(api.getSnapshot).toHaveBeenCalledWith(mockBootstrap.latestSnapshotId);
    expect(await screen.findByText(`v${mockBootstrap.appVersion}`)).toBeInTheDocument();
    // The comparison is asked for after the estate, never as part of it.
    expect(api.compareSnapshots).toHaveBeenCalled();
  });

  it("focuses the search on mod+K and opens the shortcuts on mod+/", async () => {
    render(<App />);
    await screen.findByRole("button", { name: words.nav.estate });

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(document.activeElement).toBe(screen.getByRole("combobox", { name: words.shell.search_aria }));

    fireEvent.keyDown(window, { key: "/", metaKey: true });
    expect(screen.getByRole("dialog", { name: words.shortcuts.title })).toBeVisible();
  });

  it("applies a stored explicit theme to the document", async () => {
    window.localStorage.setItem("azdocs-theme", JSON.stringify("dark"));

    render(<App />);
    await screen.findByRole("button", { name: words.nav.estate });

    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
