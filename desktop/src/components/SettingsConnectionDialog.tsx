import { useEffect, useRef } from "react";
import { Check, LoaderCircle, X } from "lucide-react";
import { useLabels } from "../labels";
import type { ConnectionCheck } from "../types";
import { PermissionStatus } from "./PermissionStatus";

type Props = {
  open: boolean;
  testing: boolean;
  tenantName: string;
  check?: ConnectionCheck;
  error?: string;
  onClose: () => void;
};

export function SettingsConnectionDialog(props: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const {
    desktop: {
      settings: { editor: words },
    },
  } = useLabels();
  useEffect(() => {
    if (props.open && !dialog.current?.open) dialog.current?.showModal();
    if (!props.open && dialog.current?.open) dialog.current?.close();
  }, [props.open]);
  return (
    <dialog
      ref={dialog}
      className="dashboard-modal settings-connection-dialog"
      aria-labelledby="connection-test-title"
      aria-describedby="connection-test-description"
      onClose={props.onClose}
      onCancel={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        if (event.key === "Escape") event.stopPropagation();
      }}
    >
      <header>
        <div>
          <h2 id="connection-test-title">{words.draft_test}</h2>
          <p id="connection-test-description">{props.tenantName}</p>
        </div>
        <button
          type="button"
          className="icon-button"
          aria-label={words.close_test}
          onClick={() => dialog.current?.close()}
          autoFocus
        >
          <X size={20} />
        </button>
      </header>
      <div className="dashboard-modal-body" aria-busy={props.testing}>
        {props.testing ? (
          <div className="settings-loading" role="status">
            <LoaderCircle className="spin" size={20} />
            {words.testing}
          </div>
        ) : props.error ? (
          <div className="settings-alert" role="alert">
            <strong>{words.test_failed}</strong>
            <p>{props.error}</p>
          </div>
        ) : props.check ? (
          <>
            <p className="settings-auth-success">
              <Check size={17} aria-hidden="true" />
              {words.authenticated}
            </p>
            <PermissionStatus check={props.check} expanded />
          </>
        ) : null}
      </div>
      <footer>
        <span>{words.test_detail}</span>
        <button
          type="button"
          className="quiet-button"
          onClick={() => dialog.current?.close()}
        >
          {words.close_test}
        </button>
      </footer>
    </dialog>
  );
}
