import { useEffect, useRef, useState } from "react";
import { RefreshCw, Square, X } from "lucide-react";
import { useLabels } from "../labels";
import { useWebsites } from "../website-capture";
import type { CollectionFeedback } from "../collection-feedback";
import type { CollectOptions } from "../api";
import type { Subscription } from "../types";
import { PermissionStatus } from "./PermissionStatus";
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
  /** Subscriptions the open snapshot knows, offered as the collection scope. */
  subscriptions: Subscription[];
  message?: string;
  error?: string;
  onClose: () => void;
  onCollect: (options: CollectOptions) => void;
  onCancel: () => void;
};

export function CollectionDialog(props: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [chosen, setChosen] = useState<string[]>([]);
  const [notes, setNotes] = useState("");
  const {
    common: { websites: words },
    desktop: { shell, collection },
  } = useLabels();
  const websites = useWebsites();
  useEffect(() => {
    if (props.open && !dialog.current?.open) dialog.current?.showModal();
    if (!props.open && dialog.current?.open) dialog.current?.close();
  }, [props.open]);
  function toggle(id: string) {
    setChosen((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );
  }
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
        {props.feedback?.permissions && (
          <PermissionStatus check={props.feedback.permissions} />
        )}
        {!props.canCollect ? <p>{props.credentialsHint}</p> : null}
        {props.message ? (
          <p className="collection-dialog-status" role="status">
            {props.message}
          </p>
        ) : null}
        {props.error ? <p role="alert">{props.error}</p> : null}
        {!props.collecting ? (
          <>
            {props.subscriptions.length > 0 ? (
              <fieldset className="collection-scope">
                <legend>{collection.scope}</legend>
                <p>{collection.scope_detail}</p>
                {props.subscriptions.map((subscription) => (
                  <label key={subscription.id}>
                    <input
                      type="checkbox"
                      checked={chosen.includes(subscription.id)}
                      onChange={() => toggle(subscription.id)}
                    />
                    <span>{subscription.displayName}</span>
                    <small className="mono">{subscription.id}</small>
                  </label>
                ))}
              </fieldset>
            ) : null}
            <label className="collection-notes">
              <span>{collection.notes}</span>
              <input
                value={notes}
                placeholder={collection.notes_placeholder}
                onChange={(event) => setNotes(event.target.value)}
              />
            </label>
            <button
              className="collect-button"
              disabled={!props.canCollect || websites?.blocked}
              onClick={() => props.onCollect({ subscriptions: chosen, notes })}
            >
              <RefreshCw size={15} />
              {props.feedback ? words.collect_again : shell.collect}
            </button>
          </>
        ) : null}
      </section>
      {props.collecting ? (
        <div className="collection-cancel">
          {props.autoCapturing ? (
            <WebsiteCaptureControls autoCapturing />
          ) : (
            <button className="quiet-button" onClick={props.onCancel}>
              <Square size={14} /> {shell.cancel_collection}
            </button>
          )}
        </div>
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
