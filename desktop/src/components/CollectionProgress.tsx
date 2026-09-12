import { useEffect, useState } from "react";
import {
  Check,
  Circle,
  Clock3,
  LoaderCircle,
  AlertTriangle,
} from "lucide-react";
import { elapsedTime, type CollectionFeedback } from "../collection-feedback";
import { fill } from "../format";
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
      <div className="collection-progress-heading" role="status">
        <strong>{words.states.running}</strong>
        <span>
          {fill(words.count_progress, {
            completed: progress.completed,
            total: progress.total,
          })}
        </span>
      </div>
      <ProgressBar
        completed={progress.completed}
        total={progress.total}
        label={words.title}
      />
      <dl className="collection-live-stats">
        <div>
          <dt>{words.saved_count}</dt>
          <dd>{progress.captured}</dd>
        </div>
        <div>
          <dt>{words.failed_count}</dt>
          <dd data-signal={progress.failed > 0 ? "warning" : undefined}>
            {progress.failed}
          </dd>
        </div>
        <div>
          <dt>{words.remaining_count}</dt>
          <dd>{Math.max(0, progress.total - progress.completed)}</dd>
        </div>
      </dl>
      {progress.url ? (
        <div className="collection-current">
          <span>{words.current_url}</span>
          <code>{progress.url}</code>
        </div>
      ) : null}
    </div>
  );
}

export function CollectionProgress({
  feedback,
}: {
  feedback: CollectionFeedback;
}) {
  const words = useLabels().common.websites;
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    setNow(Date.now());
    if (feedback.endedAt !== undefined) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [feedback.startedAt, feedback.endedAt]);
  const stages = ["inventory", "discovery", "capture"] as const;
  const failed =
    feedback.stage === "failed" || feedback.result?.status === "failed";
  const running = feedback.endedAt === undefined;
  const current =
    feedback.stage === "complete"
      ? failed
        ? 0
        : 3
      : stages.indexOf(
          feedback.stage === "failed" ? "inventory" : feedback.stage,
        );
  const titles = {
    inventory: words.stage_inventory,
    discovery: words.stage_discovery,
    capture: words.title,
  };
  const result = feedback.result;
  const issues =
    feedback.stage === "failed" ||
    (result &&
      (result.status !== "complete" ||
        Boolean(
          result.screenshots?.failed ||
          result.screenshots?.error ||
          result.screenshots?.cancelled,
        )));
  return (
    <div className="collection-feedback">
      <div className="collection-progress-heading">
        <strong>
          {feedback.stage === "complete" ? (
            <>
              {issues ? <AlertTriangle size={18} /> : <Check size={18} />}{" "}
              {words.finished}
            </>
          ) : feedback.stage === "failed" ? (
            <>
              <AlertTriangle size={18} />
              {words.failed_count}
            </>
          ) : (
            <>
              <LoaderCircle size={18} className="spin" />
              {titles[feedback.stage]}
            </>
          )}
        </strong>
        <span className="collection-elapsed">
          <Clock3 size={13} />
          {words.elapsed}{" "}
          <time>
            {elapsedTime(feedback.startedAt, feedback.endedAt ?? now)}
          </time>
        </span>
      </div>
      <ol className="collection-stages" aria-label={words.stages}>
        {stages.map((stage, index) => (
          <li
            key={stage}
            className={
              index < current
                ? "done"
                : index === current && (running || failed)
                  ? "current"
                  : ""
            }
            aria-current={index === current && running ? "step" : undefined}
          >
            {index < current ? (
              <Check size={14} />
            ) : index === current && failed ? (
              <AlertTriangle size={14} />
            ) : index === current && running ? (
              <LoaderCircle size={14} className="spin" />
            ) : (
              <Circle size={14} />
            )}
            <span>{titles[stage]}</span>
          </li>
        ))}
      </ol>
      {feedback.stage === "capture" && feedback.screenshots ? (
        <ScreenshotProgress progress={feedback.screenshots} />
      ) : null}
      {feedback.stage === "inventory" ? (
        <>
          <div className="collection-progress-heading" role="status">
            <span>{words.query_progress}</span>
            <strong>
              {feedback.queries
                ? fill(words.count_progress, {
                    completed: feedback.queries.completed,
                    total: feedback.queries.total,
                  })
                : words.preparing}
            </strong>
          </div>
          <ProgressBar
            completed={feedback.queries?.completed}
            total={feedback.queries?.total}
            label={words.query_progress}
          />
          <dl className="collection-live-stats">
            <div>
              <dt>{words.rows_count}</dt>
              <dd>{(feedback.queries?.rows ?? 0).toLocaleString()}</dd>
            </div>
            <div>
              <dt>{words.failed_count}</dt>
              <dd
                data-signal={feedback.queries?.failed ? "warning" : undefined}
              >
                {feedback.queries?.failed ?? 0}
              </dd>
            </div>
          </dl>
          <div className="collection-current">
            <span>{words.latest_query}</span>
            <p>{feedback.queries?.latestQuery ?? words.preparing}</p>
          </div>
        </>
      ) : null}
      {feedback.stage === "discovery" ||
      (feedback.stage === "capture" && !feedback.screenshots) ? (
        <>
          <ProgressBar label={titles[feedback.stage]} />
          <p>
            {feedback.stage === "discovery"
              ? words.discovering
              : words.preparing}
          </p>
        </>
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
    </div>
  );
}
