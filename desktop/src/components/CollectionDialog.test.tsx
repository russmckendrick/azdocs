import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DEFAULT_LABELS } from "../labels";
import type { ConnectionCheck } from "../types";
import { CollectionDialog } from "./CollectionDialog";
import { PermissionStatus } from "./PermissionStatus";

const words = DEFAULT_LABELS.common.websites;
const permissions: ConnectionCheck = {
  checkedAt: "2026-09-22T07:46:00Z",
  verdict: "read_only",
  subscriptions: [],
  inaccessibleSubscriptions: [],
  grants: [],
  issues: [],
};

describe("compact collection dialog", () => {
  it("keeps cancellation and errors available without repeating the phase message", () => {
    const onCancel = vi.fn();
    render(
      <CollectionDialog
        open
        collecting
        autoCapturing
        canCollect
        credentialsHint=""
        subscriptions={[]}
        feedback={{ startedAt: 0, stage: "capture" }}
        message="Repeated phase message"
        error="Capture connection lost"
        onClose={vi.fn()}
        onCollect={vi.fn()}
        onCancel={onCancel}
      />,
    );
    expect(
      screen.queryByText("Repeated phase message"),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Capture connection lost",
    );
    fireEvent.click(screen.getByRole("button", { name: words.cancel }));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("collapses successful permission evidence but opens warnings", () => {
    const { rerender } = render(
      <PermissionStatus check={permissions} compact />,
    );
    expect(
      screen.getByLabelText(DEFAULT_LABELS.common.access.title),
    ).not.toHaveAttribute("open");
    rerender(
      <PermissionStatus
        check={{
          ...permissions,
          verdict: "unable_to_verify",
          issues: [{ kind: "assignment_read_failed", scope: "subscription" }],
        }}
        compact
      />,
    );
    expect(
      screen.getByLabelText(DEFAULT_LABELS.common.access.title),
    ).toHaveAttribute("open");
    expect(
      screen.getByText(
        DEFAULT_LABELS.common.access.reasons.assignment_read_failed,
      ),
    ).toBeVisible();
  });
});
