import { useEffect, useRef } from "react";
import { RefreshCw, X } from "lucide-react";
import { useLabels } from "../labels";
import { useWebsites } from "../website-capture";
import type { CollectionFeedback } from "../collection-feedback";
import { CollectionProgress } from "./CollectionProgress";
import { WebsiteCaptureControls } from "./WebsiteScreenshots";

type Props = {
  open: boolean;
  feedback?: CollectionFeedback;
  collecting: boolean;
  autoCapturing: boolean;
  canCollect: boolean;
  credentialsHint: string;
  snapshotLabel?: string;
  message?: string;
  error?: string;
  onClose: () => void;
  onCollect: () => void;
};

export function CollectionDialog(props: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const {
    common: { websites: words },
    desktop: { shell },
  } = useLabels();
  const websites = useWebsites();
  useEffect(() => {
    if (props.open && !dialog.current?.open) dialog.current?.showModal();
    if (!props.open && dialog.current?.open) dialog.current?.close();
  }, [props.open]);
  return (
    <dialog
      ref={dialog}
      className="collection-dialog"
      aria-labelledby="collection-dialog-title"
      onClose={props.onClose}
      onKeyDown={(event) => {
        if (event.key === "Escape") event.stopPropagation();
      }}
    >
      <header>
        <h2 id="collection-dialog-title">{shell.collect}</h2>
        <button
          className="quiet-button"
          aria-label={words.close_collection}
          onClick={() => dialog.current?.close()}
        >
          <X size={18} />
        </button>
      </header>
      <section>
        {!props.collecting && !props.feedback ? (
          <>
            <h3>{words.new_snapshot}</h3>
            <p>{words.collection_detail}</p>
          </>
        ) : null}
        {props.feedback ? (
          <CollectionProgress feedback={props.feedback} />
        ) : null}
        {!props.canCollect ? <p>{props.credentialsHint}</p> : null}
        {props.message ? (
          <p className="collection-dialog-status" role="status">
            {props.message}
          </p>
        ) : null}
        {props.error ? <p role="alert">{props.error}</p> : null}
        {!props.collecting ? (
          <button
            className="collect-button"
            disabled={!props.canCollect || websites?.blocked}
            onClick={props.onCollect}
          >
            <RefreshCw size={15} />
            {props.feedback ? words.collect_again : shell.collect}
          </button>
        ) : null}
      </section>
      {props.collecting ? (
        props.autoCapturing ? (
          <div className="collection-cancel">
            <WebsiteCaptureControls autoCapturing />
          </div>
        ) : null
      ) : props.snapshotLabel ? (
        <section>
          <h3>{words.title}</h3>
          {props.snapshotLabel ? (
            <p className="collection-snapshot-label">
              {words.saved_snapshot}: {props.snapshotLabel}
            </p>
          ) : null}
          <p>{words.saved_detail}</p>
          <WebsiteCaptureControls autoCapturing={props.autoCapturing} />
        </section>
      ) : null}
      <p className="collection-dialog-note">{words.background_note}</p>
    </dialog>
  );
}
