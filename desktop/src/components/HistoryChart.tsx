import { useEffect, useRef, useState } from "react";
import { AlertTriangle, ChevronRight } from "lucide-react";
import type { SnapshotSummary } from "../types";
import {
  dateTime,
  dayMonth,
  dayMonthTime,
  fill,
  plural,
  snapshotStatusLabel,
} from "../format";
import { useLabels } from "../labels";
import {
  hasCompleteInventory,
  resourceHistoryMarkers,
  resourceHistoryScale,
} from "./dashboard-model";

export function HistoryChart({
  series,
  onSelect,
}: {
  series: SnapshotSummary[];
  onSelect: (snapshot: SnapshotSummary) => void;
}) {
  const {
    common,
    desktop: { overview: words, history },
  } = useLabels();
  const chart = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(320);
  useEffect(() => {
    const element = chart.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) =>
      setWidth(Math.max(240, entries[0].contentRect.width)),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const complete = series.filter((snapshot) =>
    hasCompleteInventory(snapshot.status),
  );
  const incomplete = series.length - complete.length;
  const scale = resourceHistoryScale(
    complete.map((snapshot) => snapshot.resources),
  );
  const tickLabels = scale.ticks.map((tick) => tick.toLocaleString());
  const startX = Math.max(
    40,
    Math.max(...tickLabels.map((label) => label.length)) * 7 + 12,
  );
  const endX = width - 12;
  const top = 12;
  const bottom = 160;
  const yPosition = (value: number) =>
    bottom - ((value - scale.min) / (scale.max - scale.min)) * (bottom - top);
  const first = Date.parse(series[0]?.createdAt ?? "");
  const last = Date.parse(series.at(-1)?.createdAt ?? "");
  const span = last - first;
  const points = series.map((snapshot) => ({
    snapshot,
    x:
      span > 0
        ? startX +
          ((Date.parse(snapshot.createdAt) - first) / span) * (endX - startX)
        : (startX + endX) / 2,
    y: yPosition(snapshot.resources),
  }));
  const inventoryPoints = points.filter((point) =>
    hasCompleteInventory(point.snapshot.status),
  );
  const path = inventoryPoints
    .map((point, index) => `${index === 0 ? "M" : "L"}${point.x},${point.y}`)
    .join(" ");
  const markers = resourceHistoryMarkers(inventoryPoints);
  const tickCount = Math.max(
    2,
    Math.min(4, Math.floor((endX - startX) / 95) + 1),
  );
  const dateTicks =
    span > 0
      ? Array.from({ length: tickCount }, (_, index) => {
          const fraction = index / (tickCount - 1);
          const date = new Date(first + span * fraction).toISOString();
          return {
            x: startX + (endX - startX) * fraction,
            label: dayMonth(date),
          };
        }).filter(
          (tick, index, ticks) =>
            index === 0 || tick.label !== ticks[index - 1].label,
        )
      : series.length
        ? [{ x: (startX + endX) / 2, label: dayMonth(series[0].createdAt) }]
        : [];
  const description = (snapshot: SnapshotSummary) =>
    [
      dateTime(snapshot.createdAt),
      fill(history.resources, { count: snapshot.resources.toLocaleString() }),
      snapshotStatusLabel(snapshot.status),
    ].join(" · ");

  return (
    <div className="dashboard-history-chart" ref={chart}>
      {complete.length > 0 && (
        <svg
          className="dashboard-history-plot"
          viewBox={`0 0 ${width} 192`}
          role="group"
          aria-label={words.growth_aria}
        >
          {scale.ticks.map((tick, index) => (
            <g key={tick}>
              <line
                x1={startX}
                x2={endX}
                y1={yPosition(tick)}
                y2={yPosition(tick)}
                className="dashboard-gridline"
              />
              <text x={startX - 10} y={yPosition(tick) + 4} textAnchor="end">
                {tickLabels[index]}
              </text>
            </g>
          ))}
          <path className="dashboard-history-line" d={path} />
          {markers.map(({ snapshot, x, y }) => (
            <g
              key={snapshot.id}
              role="button"
              tabIndex={0}
              aria-haspopup="dialog"
              aria-label={description(snapshot)}
              onClick={() => onSelect(snapshot)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(snapshot);
                }
              }}
              className="dashboard-history-point"
            >
              <title>{description(snapshot)}</title>
              <circle cx={x} cy={y} r={12} className="dashboard-point-target" />
              <circle cx={x} cy={y} r={3.5} className="dashboard-point" />
            </g>
          ))}
          {dateTicks.map((tick, index) => (
            <text
              key={tick.x}
              x={tick.x}
              y={184}
              textAnchor={
                dateTicks.length === 1
                  ? "middle"
                  : index === 0
                    ? "start"
                    : index === dateTicks.length - 1
                      ? "end"
                      : "middle"
              }
            >
              {tick.label}
            </text>
          ))}
        </svg>
      )}
      {complete.length < 2 && (
        <p className="dashboard-note">{words.dashboard.history_empty}</p>
      )}
      {series.length > 0 && (
        <details className="dashboard-history-records">
          <summary>
            <ChevronRight size={12} aria-hidden="true" />
            <span>
              {plural(words.dashboard.history_snapshots, series.length)}
            </span>
            {incomplete > 0 && (
              <span className="dashboard-history-incomplete">
                <AlertTriangle size={12} aria-hidden="true" />
                {fill(words.dashboard.history_incomplete, {
                  count: incomplete,
                })}
              </span>
            )}
          </summary>
          <ul>
            {[...series].reverse().map((snapshot) => (
              <li key={snapshot.id}>
                <button
                  onClick={() => onSelect(snapshot)}
                  aria-label={description(snapshot)}
                  aria-haspopup="dialog"
                >
                  <time dateTime={snapshot.createdAt}>
                    {dayMonthTime(snapshot.createdAt)}
                  </time>
                  <strong>
                    {hasCompleteInventory(snapshot.status)
                      ? snapshot.resources.toLocaleString()
                      : common.verdict.none}
                  </strong>
                  <span
                    data-incomplete={
                      !hasCompleteInventory(snapshot.status) || undefined
                    }
                  >
                    {snapshotStatusLabel(snapshot.status)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}
