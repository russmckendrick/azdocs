import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { X } from "lucide-react";
import { LOCATION_ICON } from "../azure-icons";
import { displayLocation } from "../azure-values";
import { fill } from "../format";
import { useLabels } from "../labels";
import type { DashboardDestination, EstateSnapshot, RegionPoint } from "../types";
import { EmptyState, ViewHeading } from "./view-chrome";
import { MapControls } from "./MapControls";
import { applyCamera, useMapCamera } from "./use-map-camera";
import { LAND_PATH, MAP_HEIGHT, MAP_WIDTH, project } from "./world-map";

/** A catalogued Azure region placed on the page's map, in pixels. */
interface RegionMark {
  /** The stored location code, as the estate filter expects it. */
  name: string;
  label: string;
  physicalLocation: string;
  geography: string;
  point: RegionPoint;
  count: number;
  x: number;
  y: number;
  radius: number;
}

/** What the details dialog shows for any datacentre, in use or not. */
interface RegionDetail {
  key: string;
  /** The stored location code when the snapshot uses the region. */
  storedName: string | null;
  label: string;
  point: RegionPoint;
  count: number;
}

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

const LABEL_HEIGHT = 14;
const LABEL_GAP = 6;
const CHAR_WIDTH = 6.3;
/** Every active region is the same small marker; the count is in its label. */
const MARK_RADIUS = 4;

function overlaps(a: Box, b: Box) {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
}

type Side = "right" | "right-below" | "right-above" | "left" | "left-below" | "left-above" | "below" | "above";

interface Placement {
  mark: RegionMark;
  count: string;
  text: string;
  box: Box;
  side: Side;
  /** Where a leader line meets the label; only drawn off the default side. */
  anchor: [number, number];
}

/**
 * Greedy label placement. Labels prefer the right of their marker, then
 * shift a row down or up while staying on the right, and only then flip to
 * the left or stack above/below — a label to the left of one marker reads as
 * belonging to the next marker along, which is exactly the confusion a
 * crowded coast (UK South beside West Europe) produced. The largest markers
 * choose first, every label avoids the other markers, and any label off the
 * default side is joined to its marker by a leader line. Widths are
 * estimated from the character count; the halo behind the text absorbs the
 * error.
 */
function placeLabels(marks: RegionMark[], width: number, height: number): Placement[] {
  const taken: Box[] = marks.map((mark) => ({
    x: mark.x - mark.radius,
    y: mark.y - mark.radius,
    w: mark.radius * 2,
    h: mark.radius * 2,
  }));
  return marks.map((mark) => {
    const count = mark.count.toLocaleString();
    const text = `${mark.label} · ${count}`;
    const w = text.length * CHAR_WIDTH + 6;
    const h = LABEL_HEIGHT;
    const right = mark.x + mark.radius + LABEL_GAP;
    const left = mark.x - mark.radius - LABEL_GAP - w;
    const row = mark.y - h / 2;
    const step = h + 2;
    const candidates: Array<{ side: Side; box: Box }> = [
      { side: "right", box: { x: right, y: row, w, h } },
      { side: "right-below", box: { x: right, y: row + step, w, h } },
      { side: "right-above", box: { x: right, y: row - step, w, h } },
      { side: "left", box: { x: left, y: row, w, h } },
      { side: "left-below", box: { x: left, y: row + step, w, h } },
      { side: "left-above", box: { x: left, y: row - step, w, h } },
      { side: "below", box: { x: mark.x - w / 2, y: mark.y + mark.radius + LABEL_GAP, w, h } },
      { side: "above", box: { x: mark.x - w / 2, y: mark.y - mark.radius - LABEL_GAP - h, w, h } },
    ];
    const chosen =
      candidates.find(
        ({ box }) =>
          box.x >= 0 &&
          box.y >= 0 &&
          box.x + box.w <= width &&
          box.y + box.h <= height &&
          !taken.some((used) => overlaps(used, box)),
      ) ?? candidates[0];
    taken.push(chosen.box);
    const { box, side } = chosen;
    const anchor: [number, number] = side.startsWith("right")
      ? [box.x - 2, box.y + h / 2]
      : side.startsWith("left")
        ? [box.x + box.w + 2, box.y + h / 2]
        : side === "below"
          ? [box.x + box.w / 2, box.y]
          : [box.x + box.w / 2, box.y + h];
    return { mark, count, text, box, side, anchor };
  });
}

/** The point on a marker's edge facing a label anchor, for its leader line. */
function edgeTowards(mark: RegionMark, [x, y]: [number, number]): [number, number] {
  const dx = x - mark.x;
  const dy = y - mark.y;
  const distance = Math.hypot(dx, dy) || 1;
  return [mark.x + (dx / distance) * mark.radius, mark.y + (dy / distance) * mark.radius];
}

/**
 * The facts Microsoft publishes about one datacentre, plus how much of this
 * snapshot lives there. Regions in use get an action to open their resources.
 */
function RegionDialog({
  detail,
  onClose,
  onOpenResources,
}: {
  detail: RegionDetail;
  onClose: () => void;
  onOpenResources?: () => void;
}) {
  const {
    common,
    desktop: { regions: words },
  } = useLabels();
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const { point } = detail;
  const coordinates = fill(words.coordinate_format, {
    latitude: Math.abs(point.latitude).toFixed(2),
    ns: point.latitude >= 0 ? words.north : words.south,
    longitude: Math.abs(point.longitude).toFixed(2),
    ew: point.longitude >= 0 ? words.east : words.west,
  });
  const facts: Array<[string, ReactNode]> = [
    [words.programmatic_name, <code key="name">{detail.key}</code>],
    [words.physical_location, point.physicalLocation],
    [words.geography, point.geography],
    [words.coordinates, coordinates],
    [words.availability_zones, point.availabilityZones ? words.zones_available : words.zones_unavailable],
    [words.paired_region, point.pairedRegion ?? common.verdict.none],
    [words.opened, point.yearOpened ?? common.verdict.none],
    [words.status, point.open ? words.status_open : words.status_announced],
    [words.data_residency, point.dataResidency ?? common.verdict.none],
    [words.resources_here, detail.count.toLocaleString()],
  ];
  return (
    <dialog
      ref={dialog}
      className="dashboard-modal region-dialog"
      aria-label={words.detail_aria}
      onClose={onClose}
      onClick={(event) => {
        if (event.target === dialog.current) onClose();
      }}
    >
      <header>
        <img src={LOCATION_ICON} alt="" />
        <div>
          <h2>{detail.label}</h2>
          <p>
            {point.physicalLocation} · {point.geography}
          </p>
        </div>
        <button className="icon-button" onClick={onClose} aria-label={words.close}>
          <X size={18} />
        </button>
      </header>
      <div className="dashboard-modal-body">
        <dl className="region-facts">
          {facts.map(([term, value]) => (
            <div key={term}>
              <dt>{term}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
      </div>
      <footer>
        {onOpenResources ? (
          <button className="collect-button" onClick={onOpenResources}>
            {words.open_resources}
          </button>
        ) : null}
        <button className="quiet-button" onClick={onClose}>
          {words.close}
        </button>
      </footer>
    </dialog>
  );
}

/**
 * Every Azure datacentre the catalogue knows, with the regions this snapshot
 * uses highlighted as small pulsing markers and labelled with their count.
 * Any marker opens the region's details; a table row opens the Estate
 * filtered to that region — a list of what is there, rather than a dialog
 * repeating it.
 */
export function RegionsView({
  estate,
  onOpenResults,
}: {
  estate: EstateSnapshot;
  onOpenResults: (destination: DashboardDestination) => void;
}) {
  const {
    desktop: { regions: words, overview },
  } = useLabels();
  const host = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(960);
  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) =>
      setWidth(Math.max(320, Math.min(1400, Math.floor(entries[0].contentRect.width)))),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const height = Math.round((width * MAP_HEIGHT) / MAP_WIDTH);
  const scale = width / MAP_WIDTH;
  const { camera, panning, zoomIn, zoomOut, reset, consumedByDrag, canZoomIn, canZoomOut, pointerHandlers } =
    useMapCamera(width, height);
  const [selected, setSelected] = useState<RegionDetail | null>(null);
  const catalogue = estate.azureMetadata.regions;
  const nameOf = (location: string) =>
    displayLocation(estate.azureMetadata, location, overview.location_not_stored);

  const { active, quiet, unplaced } = useMemo(() => {
    const active: RegionMark[] = [];
    const unplaced: string[] = [];
    const used = new Set<string>();
    for (const location of estate.locations) {
      const key = location.name.toLowerCase();
      const region = catalogue[key];
      if (!region) {
        unplaced.push(nameOf(location.name));
        continue;
      }
      used.add(key);
      const [x, y] = project(region.longitude, region.latitude);
      active.push({
        name: location.name,
        label: nameOf(location.name),
        physicalLocation: region.physicalLocation,
        geography: region.geography,
        point: region,
        count: location.count,
        x: x * scale,
        y: y * scale,
        radius: MARK_RADIUS,
      });
    }
    active.sort((a, b) => b.count - a.count || a.label.localeCompare(b.label));
    const quiet = Object.entries(catalogue)
      .filter(([key]) => !used.has(key))
      .map(([key, region]) => {
        const [x, y] = project(region.longitude, region.latitude);
        return {
          key,
          label: nameOf(key),
          physicalLocation: region.physicalLocation,
          point: region,
          x: x * scale,
          y: y * scale,
        };
      });
    return { active, quiet, unplaced };
    // nameOf closes over estate.azureMetadata, which is part of `estate`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [estate, catalogue, scale]);
  const viewed = active.map((mark) => {
    const [x, y] = applyCamera(camera, mark.x, mark.y);
    return { ...mark, x, y };
  });
  const labels = placeLabels(viewed, width, height);
  const totalCount = active.reduce((sum, mark) => sum + mark.count, 0);

  function openResources(mark: { name: string; label: string }) {
    setSelected(null);
    onOpenResults({
      view: "estate",
      label: mark.label,
      filter: { location: mark.name },
    });
  }
  function show(detail: RegionDetail) {
    if (consumedByDrag()) return;
    setSelected(detail);
  }

  return (
    <div className="regions-workspace">
      <ViewHeading
        title={words.title}
        description={fill(words.description, {
          active: active.length,
          total: Object.keys(catalogue).length,
        })}
      />
      <section className="regions-map-panel" aria-label={words.title}>
        <div className="regions-map" ref={host}>
          <svg
            width={width}
            height={height}
            role="img"
            aria-label={words.map_aria}
            className={camera.k > 1 ? (panning ? "map-panning" : "map-pannable") : undefined}
            {...pointerHandlers}
          >
            <g transform={`translate(${camera.tx} ${camera.ty}) scale(${camera.k * scale})`}>
              <path className="regions-map-land" d={LAND_PATH} fillRule="evenodd" />
            </g>
            {quiet.map((region) => {
              const [x, y] = applyCamera(camera, region.x, region.y);
              const detail: RegionDetail = {
                key: region.key,
                storedName: null,
                label: region.label,
                point: region.point,
                count: 0,
              };
              return (
                <g
                  key={region.key}
                  className="regions-mark regions-mark-other"
                  role="button"
                  tabIndex={0}
                  aria-haspopup="dialog"
                  aria-label={fill(words.show_region, { name: region.label })}
                  onClick={() => show(detail)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      show(detail);
                    }
                  }}
                >
                  <title>
                    {region.label} · {region.physicalLocation}
                  </title>
                  <circle className="regions-mark-target" cx={x} cy={y} r={11} />
                  <circle className="regions-mark-quiet" cx={x} cy={y} r={3} />
                </g>
              );
            })}
            {labels.map(({ mark, count, text, box, side, anchor }) => {
              const [edgeX, edgeY] = edgeTowards(mark, anchor);
              const detail: RegionDetail = {
                key: mark.name.toLowerCase(),
                storedName: mark.name,
                label: mark.label,
                point: mark.point,
                count: mark.count,
              };
              return (
                <g
                  key={mark.name}
                  className="regions-mark"
                  role="button"
                  tabIndex={0}
                  aria-haspopup="dialog"
                  aria-label={fill(words.show_region, { name: mark.label })}
                  onClick={() => show(detail)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      show(detail);
                    }
                  }}
                >
                  <title>
                    {mark.label} · {mark.physicalLocation} · {count}
                  </title>
                  {side !== "right" ? (
                    <line
                      className="regions-leader"
                      x1={edgeX}
                      y1={edgeY}
                      x2={anchor[0]}
                      y2={anchor[1]}
                    />
                  ) : null}
                  <circle className="regions-mark-target" cx={mark.x} cy={mark.y} r={mark.radius + 8} />
                  <circle className="map-pulse" cx={mark.x} cy={mark.y} r={mark.radius} />
                  <circle className="regions-mark-active" cx={mark.x} cy={mark.y} r={mark.radius} />
                  <text className="regions-label" x={box.x} y={box.y + LABEL_HEIGHT - 3}>
                    {text}
                  </text>
                </g>
              );
            })}
          </svg>
          <MapControls
            onZoomIn={zoomIn}
            onZoomOut={zoomOut}
            onReset={reset}
            canZoomIn={canZoomIn}
            canZoomOut={canZoomOut}
          />
        </div>
        <div className="regions-legend">
          <span>
            <i className="active" />
            {words.legend_active}
          </span>
          <span>
            <i className="quiet" />
            {words.legend_quiet}
          </span>
        </div>
      </section>
      {active.length ? (
        <div className="data-grid-wrap regions-table-wrap">
          <table className="data-grid regions-table">
            <thead>
              <tr>
                <th>{words.column_region}</th>
                <th>{words.column_geography}</th>
                <th className="numeric">{words.column_resources}</th>
                <th>{words.column_share}</th>
              </tr>
            </thead>
            <tbody>
              {active.map((mark) => (
                <tr key={mark.name}>
                  <td>
                    <button
                      className="regions-open"
                      onClick={() => openResources(mark)}
                      aria-label={fill(words.open_region, { name: mark.label })}
                    >
                      <img src={LOCATION_ICON} alt="" />
                      <span>
                        {mark.label}
                        <small>{mark.physicalLocation}</small>
                      </span>
                    </button>
                  </td>
                  <td className="secondary">{mark.geography}</td>
                  <td className="numeric emphatic">{mark.count.toLocaleString()}</td>
                  <td className="share">
                    <span className="regions-share" aria-hidden="true">
                      <span style={{ width: `${(mark.count / Math.max(1, totalCount)) * 100}%` }} />
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <EmptyState className="no-results" title={words.empty_title} detail={words.empty_detail} />
      )}
      {unplaced.length ? (
        <p className="dashboard-note">{fill(words.unplaced, { names: unplaced.join(", ") })}</p>
      ) : null}
      {selected ? (
        <RegionDialog
          detail={selected}
          onClose={() => setSelected(null)}
          onOpenResources={
            selected.storedName === null
              ? undefined
              : () => openResources({ name: selected.storedName as string, label: selected.label })
          }
        />
      ) : null}
    </div>
  );
}
