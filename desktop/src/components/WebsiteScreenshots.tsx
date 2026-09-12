import { useEffect, useRef, useState } from "react";
import { Camera, Download, Expand, RefreshCw, X } from "lucide-react";
import { ScreenshotProgress } from "./CollectionProgress";
import { getWebsiteImage, saveWebsiteImage } from "../api";
import { useLabels } from "../labels";
import { fill, dateTime } from "../format";
import { useWebsites } from "../website-capture";
import type { WebsiteCapture, WebsiteEndpoint } from "../types";

export function WebsiteCaptureControls({
  autoCapturing,
}: {
  autoCapturing: boolean;
}) {
  const context = useWebsites();
  const words = useLabels().common.websites;
  if (!context) return null;
  const { state, busy, blocked, progress, message, error, capture, cancel } =
    context;
  return (
    <div className="website-capture-controls">
      <div className="website-capture-actions">
        {!autoCapturing ? (
          <button
            className="quiet-button"
            disabled={
              blocked || !state?.endpoints.some((e) => e.status === "ready")
            }
            onClick={() => void capture()}
          >
            {words.capture_all}
          </button>
        ) : null}
        {!autoCapturing ? (
          <button
            className="quiet-button"
            disabled={
              blocked || !state?.endpoints.some((e) => e.status === "ready")
            }
            onClick={() => void capture([], true)}
          >
            {words.retry_all}
          </button>
        ) : null}
        {busy || autoCapturing ? (
          <button className="quiet-button" onClick={() => void cancel()}>
            <X size={14} /> {words.cancel}
          </button>
        ) : null}
      </div>
      {!state?.endpoints.length && !autoCapturing && !blocked ? (
        <p>{words.empty}</p>
      ) : null}
      {progress ? (
        <ScreenshotProgress progress={progress} />
      ) : message ? (
        <p role="status">{message}</p>
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
      {state?.evidenceErrors.length ? (
        <details>
          <summary>{words.missing_evidence}</summary>
          {state.evidenceErrors.map((error) => (
            <p key={error}>{error}</p>
          ))}
        </details>
      ) : null}
    </div>
  );
}

export function WebsiteScreenshots({
  resourceId,
  snapshotId,
}: {
  resourceId: string;
  snapshotId: string;
}) {
  const context = useWebsites();
  const words = useLabels().common.websites;
  const endpoints = context?.state?.endpoints.filter(
    (e) => e.resourceId === resourceId,
  );
  if (!context || !endpoints?.length) return null;
  const seen = new Set<string>();
  const unique = endpoints.filter((e) => {
    const key = `${e.url ?? e.hostname ?? e.source}:${e.status}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  return (
    <section className="resource-record-section website-section">
      <div className="resource-record-section-heading">
        <div>
          <h2>{words.title}</h2>
          <p>{words.detail}</p>
        </div>
        <Camera size={18} />
      </div>
      <div className="website-cards">
        {unique.map((endpoint) => (
          <WebsiteCard
            key={`${endpoint.url}-${endpoint.source}-${endpoint.hostname}`}
            endpoint={endpoint}
            snapshotId={snapshotId}
            capture={context.state?.captures.find(
              (c) => c.url === endpoint.url,
            )}
          />
        ))}
      </div>
    </section>
  );
}

function WebsiteCard({
  endpoint,
  capture,
  snapshotId,
}: {
  endpoint: WebsiteEndpoint;
  capture?: WebsiteCapture;
  snapshotId: string;
}) {
  const context = useWebsites();
  const words = useLabels().common.websites;
  const [image, setImage] = useState<string | null>(null);
  const [error, setError] = useState<string>();
  const [saved, setSaved] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const card = useRef<HTMLElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const element = card.current;
    if (!element) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    let live = true;
    setImage(null);
    setSaved(false);
    setError(undefined);
    if (visible && endpoint.url && capture?.capturedAt) {
      void getWebsiteImage(snapshotId, endpoint.url)
        .then((data) => {
          if (live) setImage(data);
        })
        .catch((error) => {
          if (live) setError(String(error));
        });
    }
    return () => {
      live = false;
    };
  }, [snapshotId, endpoint.url, capture?.capturedAt, visible]);
  async function save() {
    if (!endpoint.url) return;
    try {
      setSaved(await saveWebsiteImage(snapshotId, endpoint.url));
    } catch (caught) {
      setError(String(caught));
    }
  }
  const status =
    capture?.status ??
    (endpoint.status === "ready" ? "pending" : endpoint.status);
  const caption = fill(words.caption, {
    url: endpoint.url ?? "",
    time: dateTime(capture?.capturedAt),
  });
  return (
    <article ref={card} className="website-card">
      <header>
        <code>{endpoint.url ?? endpoint.hostname ?? endpoint.source}</code>
        <span>{words.states[status]}</span>
      </header>
      {image ? (
        <button
          ref={trigger}
          className="website-image-button"
          onClick={() => dialog.current?.showModal()}
          aria-label={words.view}
        >
          <img src={image} alt={caption} width={1440} height={900} />
          <span>
            <Expand size={14} /> {words.view}
          </span>
        </button>
      ) : (
        <p className="website-image-empty">
          {endpoint.status === "missing_evidence"
            ? words.missing_evidence
            : words.no_image}
        </p>
      )}
      <div className="website-card-meta">
        {capture?.capturedAt ? (
          <p>
            {fill(words.captured_at, { time: dateTime(capture.capturedAt) })}
          </p>
        ) : null}
        {capture?.finalUrl ? (
          <p>
            {words.final_url}: <code>{capture.finalUrl}</code>
          </p>
        ) : null}
        {capture?.renderer ? (
          <p>
            {words.renderer}: {capture.renderer}
          </p>
        ) : null}
        {capture?.status !== "captured" && capture?.capturedAt ? (
          <p>{words.previous_image}</p>
        ) : null}
        {capture?.attemptedAt ? (
          <p>
            {fill(words.attempted_at, { time: dateTime(capture.attemptedAt) })}
          </p>
        ) : null}
        {capture?.error ? <p>{capture.error}</p> : null}
        {error ? <p role="alert">{error}</p> : null}
        {saved ? <p role="status">{words.saved}</p> : null}
      </div>
      <div className="website-card-actions">
        <button
          className="quiet-button"
          disabled={context?.blocked || endpoint.status !== "ready"}
          onClick={() => endpoint.url && void context?.capture([endpoint.url])}
        >
          <RefreshCw size={14} />{" "}
          {capture?.capturedAt ? words.refresh : words.capture}
        </button>
        <button
          className="quiet-button"
          disabled={!image}
          onClick={() => void save()}
        >
          <Download size={14} /> {words.save}
        </button>
      </div>
      <dialog
        ref={dialog}
        className="website-image-dialog"
        aria-label={words.view}
        onClose={() => trigger.current?.focus()}
        onKeyDown={(event) => {
          if (event.key === "Escape") event.stopPropagation();
        }}
      >
        <form method="dialog">
          <strong>{endpoint.url}</strong>
          <button className="quiet-button" aria-label={words.close}>
            <X size={18} />
          </button>
        </form>
        {image ? (
          <img src={image} alt={caption} width={1440} height={900} />
        ) : null}
        <p>{caption}</p>
      </dialog>
    </article>
  );
}
