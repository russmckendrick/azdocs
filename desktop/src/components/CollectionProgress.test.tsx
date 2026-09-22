import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DEFAULT_LABELS } from "../labels";
import { CollectionProgress, ScreenshotProgress } from "./CollectionProgress";

const words = DEFAULT_LABELS.common.websites;
const progress = {
  completed: 0,
  total: 2,
  url: "https://example.test/",
  captured: 0,
  failed: 0,
  cancelled: false,
};

describe("website capture preview", () => {
  it("replaces loading with the current inline image and clears it for the next website", () => {
    const { rerender } = render(<ScreenshotProgress progress={progress} />);
    expect(screen.getByText(words.preview_loading)).toBeInTheDocument();
    rerender(
      <ScreenshotProgress
        progress={{
          ...progress,
          previewImage: "data:image/jpeg;base64,preview",
        }}
      />,
    );
    expect(screen.getByRole("img", { name: progress.url })).toHaveAttribute(
      "src",
      "data:image/jpeg;base64,preview",
    );
    expect(screen.queryByText(words.preview_loading)).not.toBeInTheDocument();
    rerender(
      <ScreenshotProgress
        progress={{
          ...progress,
          url: "https://next.test/",
          previewImage: null,
        }}
      />,
    );
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(screen.getByText(words.preview_loading)).toBeInTheDocument();
  });

  it("shows completed with warnings and retains the failed check count", () => {
    render(
      <CollectionProgress
        feedback={{
          startedAt: 0,
          endedAt: 2000,
          stage: "complete",
          result: {
            snapshotId: "snapshot",
            status: "warnings",
            queriesRun: 148,
            queriesFailed: 1,
            rowsIngested: 200,
          },
        }}
      />,
    );
    expect(
      screen.getByText(DEFAULT_LABELS.common.snapshot_warnings),
    ).toBeInTheDocument();
    expect(screen.getByText("147")).toBeInTheDocument();
    expect(screen.getByText("1")).toHaveAttribute("data-signal", "warning");
  });

  it("retains screenshot errors when the collection has finished", () => {
    render(
      <CollectionProgress
        feedback={{
          startedAt: 0,
          endedAt: 2000,
          stage: "complete",
          result: {
            snapshotId: "snapshot",
            status: "complete",
            queriesRun: 148,
            queriesFailed: 0,
            rowsIngested: 200,
            screenshots: {
              captured: 3,
              failed: 1,
              skipped: 0,
              cancelled: false,
              error: "Capture failed to start",
            },
          },
        }}
      />,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Capture failed to start",
    );
  });
});
