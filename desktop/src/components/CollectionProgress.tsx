import { useEffect, useState } from "react";
import {
  Check,
  Circle,
  Clock3,
  LoaderCircle,
  AlertTriangle,
} from "lucide-react";
import { elapsedTime, type CollectionFeedback } from "../collection-feedback";
import { fill, snapshotStatusLabel } from "../format";
import { useLabels } from "../labels";
import type { WebsiteProgress } from "../types";

function ProgressBar({
  completed,
  total,
  label,
}: {
  completed?: number;
  total?: number;
  label: string;
}) {
  const determined =
    completed !== undefined && total !== undefined && total > 0;
  return (
    <div
      className={`collection-progress-track${determined ? "" : " indeterminate"}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={determined ? 0 : undefined}
      aria-valuemax={determined ? total : undefined}
      aria-valuenow={determined ? completed : undefined}
    >
      <span
        style={
          determined
            ? {
                transform: `scaleX(${Math.max(0, Math.min(1, completed / total))})`,
              }
            : undefined
        }
      />
    </div>
  );
}

export function ScreenshotProgress({
  progress,
}: {
  progress: WebsiteProgress;
}) {
  const words = useLabels().common.websites;
  return (
    <div className="capture-live-progress">
      <div className="collection-progress-summary">
        <span role="status">
          {fill(words.count_progress, {
            completed: progress.completed,
            total: progress.total,
          })}
        </span>
        <span>
          {words.failed_count}{" "}
          <strong data-signal={progress.failed > 0 ? "warning" : undefined}>
            {progress.failed}
          </strong>
        </span>
      </div>
      <ProgressBar
        completed={progress.completed}
        total={progress.total}
        label={words.title}
      />
      {progress.url ? (
        <figure
          className="capture-inline-preview"
          aria-label={words.capture_preview}
        >
          <figcaption title={progress.url}>
            <code>{progress.url}</code>
          </figcaption>
          <div className="capture-preview-surface">
            {progress.previewImage ? (
              <img
                src={progress.previewImage}
                alt={progress.url}
                width={720}
                height={450}
              />
            ) : (
              <span className="capture-preview-loading" role="status">
                {words.preview_loading}
              </span>
            )}
          </div>
        </figure>
      ) : null}
    </div>
  );
}

export function CollectionProgress({
  feedback,
}: {
  feedback: CollectionFeedback;
}) {
  const {
    common: { websites: words },
    desktop: { shell },
  } = useLabels();
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    if (feedback.endedAt !== undefined) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [feedback.startedAt, feedback.endedAt]);
  const stages = ["inventory", "discovery", "capture"] as const;
  const result = feedback.result;
  const failed = feedback.stage === "failed" || result?.status === "failed";
  const cancelled =
    feedback.stage === "cancelled" || result?.status === "cancelled";
  const running = feedback.endedAt === undefined && !failed && !cancelled;
  const current = stages.indexOf(feedback.stage as (typeof stages)[number]);
  const titles = {
    inventory: words.stage_inventory,
    discovery: words.stage_discovery,
    capture: words.title,
  };
  const issues =
    failed ||
    cancelled ||
    Boolean(
      result &&
      (result.status !== "complete" ||
        result.screenshots?.failed ||
        result.screenshots?.error ||
        result.screenshots?.cancelled),
    );
  return (
    <div className="collection-feedback">
      <div className="collection-progress-heading">
        {running ? (
          <ol className="collection-stages" aria-label={words.stages}>
            {stages.map((stage, index) => (
              <li
                key={stage}
                className={
                  index < current ? "done" : index === current ? "current" : ""
                }
                aria-current={index === current ? "step" : undefined}
              >
                {index < current ? (
                  <Check size={12} />
                ) : index === current ? (
                  <LoaderCircle size={12} className="spin" />
                ) : (
                  <Circle size={12} />
                )}
                <span>{titles[stage]}</span>
              </li>
            ))}
          </ol>
        ) : (
          <strong role="status">
            {issues ? <AlertTriangle size={14} /> : <Check size={14} />}
            {failed
              ? words.failed_count
              : cancelled
                ? words.stage_cancelled
                : result?.status === "warnings"
                  ? snapshotStatusLabel(result.status)
                  : result?.status === "partial"
                    ? fill(shell.status_snapshot, { status: result.status })
                    : words.finished}
          </strong>
        )}
        <span
          className="collection-elapsed"
          aria-label={words.elapsed}
          title={words.elapsed}
        >
          <Clock3 size={12} />
          <time>
            {elapsedTime(feedback.startedAt, feedback.endedAt ?? now)}
          </time>
        </span>
      </div>
      {feedback.stage === "capture" && feedback.screenshots ? (
        <ScreenshotProgress progress={feedback.screenshots} />
      ) : null}
      {feedback.stage === "inventory" ? (
        <>
          <div className="collection-progress-summary">
            <span role="status">
              {feedback.queries
                ? fill(words.count_progress, {
                    completed: feedback.queries.completed,
                    total: feedback.queries.total,
                  })
                : words.preparing}
            </span>
            <span>
              {words.rows_count}{" "}
              <strong>{(feedback.queries?.rows ?? 0).toLocaleString()}</strong>
            </span>
            <span>
              {words.failed_count}{" "}
              <strong
                data-signal={feedback.queries?.failed ? "warning" : undefined}
              >
                {feedback.queries?.failed ?? 0}
              </strong>
            </span>
          </div>
          <ProgressBar
            completed={feedback.queries?.completed}
            total={feedback.queries?.total}
            label={words.query_progress}
          />
          {feedback.queries?.latestQuery ? (
            <p
              className="collection-current-query"
              title={feedback.queries.latestQuery}
            >
              {feedback.queries.latestQuery}
            </p>
          ) : null}
        </>
      ) : null}
      {feedback.stage === "discovery" ||
      (feedback.stage === "capture" && !feedback.screenshots) ? (
        <ProgressBar label={titles[feedback.stage]} />
      ) : null}
      {result ? (
        <dl className="collection-live-stats completion-stats">
          <div>
            <dt>{words.rows_count}</dt>
            <dd>{result.rowsIngested.toLocaleString()}</dd>
          </div>
          <div>
            <dt>{words.query_progress}</dt>
            <dd>
              {result.queriesRun - result.queriesFailed}
              <small> / {result.queriesRun}</small>
            </dd>
          </div>
          <div>
            <dt>{words.saved_count}</dt>
            <dd>{result.screenshots?.captured ?? 0}</dd>
          </div>
          <div>
            <dt>{words.failed_count}</dt>
            <dd data-signal={issues ? "warning" : undefined}>
              {result.queriesFailed + (result.screenshots?.failed ?? 0)}
            </dd>
          </div>
        </dl>
      ) : null}
      {result?.screenshots?.error ? (
        <p role="alert">{result.screenshots.error}</p>
      ) : result?.screenshots?.cancelled ? (
        <p>{words.cancelled}</p>
      ) : null}
    </div>
  );
}
