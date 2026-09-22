import { LAND_RINGS } from "./land-outline";

/**
 * The Natural Earth I projection (Šavrič, Jenny et al.), the pseudocylindrical
 * projection d3-geo ships as `geoNaturalEarth1`: gentle and compact, with
 * none of Mercator's ballooning at the latitudes Azure regions occupy. The
 * land outline is projected here at render time so the generated data stays
 * projection-agnostic, and region markers go through the same function so
 * they land on the same coastline.
 */
const RADIANS = Math.PI / 180;

function naturalEarth(longitude: number, latitude: number): [number, number] {
  const lambda = longitude * RADIANS;
  const phi = latitude * RADIANS;
  const phi2 = phi * phi;
  const phi4 = phi2 * phi2;
  const x =
    lambda *
    (0.8707 - 0.131979 * phi2 + phi4 * (-0.013791 + phi4 * (0.003971 * phi2 - 0.001529 * phi4)));
  const y =
    phi * (1.007226 + phi2 * (0.015085 + phi4 * (-0.044475 + 0.028874 * phi2 - 0.005916 * phi4)));
  // Screen y grows downwards.
  return [x, -y];
}

/** The map is 400 units wide so one unit is about one pixel in a panel. */
export const MAP_WIDTH = 400;
// No Azure region is south of Chile or north of Norway; cropping the poles
// keeps the map short enough to sit above a list.
const LATITUDE_TOP = 74;
const LATITUDE_BOTTOM = -56;

const [X_MIN] = naturalEarth(-180, 0);
const [X_MAX] = naturalEarth(180, 0);
const [, Y_TOP] = naturalEarth(0, LATITUDE_TOP);
const [, Y_BOTTOM] = naturalEarth(0, LATITUDE_BOTTOM);
const SCALE = MAP_WIDTH / (X_MAX - X_MIN);

export const MAP_HEIGHT = Math.round((Y_BOTTOM - Y_TOP) * SCALE);

/** Map units for a coordinate; may fall outside the frame near the poles. */
export function project(longitude: number, latitude: number): [number, number] {
  const [x, y] = naturalEarth(longitude, latitude);
  return [(x - X_MIN) * SCALE, (y - Y_TOP) * SCALE];
}

function ringPath(ring: ReadonlyArray<number>) {
  let path = "";
  for (let index = 0; index < ring.length; index += 2) {
    const [x, y] = project(ring[index], ring[index + 1]);
    path += `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
  }
  return `${path}Z`;
}

/** Every landmass as one path; holes (inland seas) rely on the even-odd rule. */
export const LAND_PATH = LAND_RINGS.map(ringPath).join("");

/** Geographic reference lines use the land's projection and camera. */
function graticulePath() {
  const lines: string[] = [];
  for (let longitude = -180; longitude <= 180; longitude += 30) {
    const points: string[] = [];
    for (let latitude = LATITUDE_BOTTOM; latitude <= LATITUDE_TOP; latitude += 2) {
      const [x, y] = project(longitude, latitude);
      points.push(`${points.length ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`);
    }
    lines.push(points.join(""));
  }
  for (let latitude = -30; latitude <= 60; latitude += 30) {
    const [left, y] = project(-180, latitude);
    const [right] = project(180, latitude);
    lines.push(`M${left.toFixed(1)},${y.toFixed(1)}H${right.toFixed(1)}`);
  }
  return lines.join("");
}

export const GRATICULE_PATH = graticulePath();

/** The globe's outline within the cropped latitudes, filled as sea. */
function seaPath() {
  const points: string[] = [];
  const edge = (longitude: number, from: number, to: number) => {
    const step = from < to ? 2 : -2;
    for (let latitude = from; step > 0 ? latitude <= to : latitude >= to; latitude += step) {
      const [x, y] = project(longitude, latitude);
      points.push(`${points.length ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`);
    }
  };
  edge(-180, LATITUDE_BOTTOM, LATITUDE_TOP);
  edge(180, LATITUDE_TOP, LATITUDE_BOTTOM);
  return `${points.join("")}Z`;
}

export const SEA_PATH = seaPath();

/**
 * Nudges markers apart so neighbouring regions (UK South beside West Europe)
 * stay separately clickable. Each marker keeps its true anchor; callers draw
 * a leader from the anchor to any marker that moved. Deterministic: markers
 * are relaxed in input order and coincident ones split on a fixed angle.
 */
export function spreadMarkers<T extends { x: number; y: number }>(
  markers: ReadonlyArray<T>,
  gap: number,
): Array<T & { anchorX: number; anchorY: number }> {
  const placed = markers.map((marker) => ({ ...marker, anchorX: marker.x, anchorY: marker.y }));
  for (let pass = 0; pass < 80; pass += 1) {
    let moved = false;
    for (let a = 0; a < placed.length; a += 1) {
      for (let b = a + 1; b < placed.length; b += 1) {
        const first = placed[a];
        const second = placed[b];
        let dx = second.x - first.x;
        let dy = second.y - first.y;
        let distance = Math.hypot(dx, dy);
        if (distance >= gap) continue;
        if (distance < 0.01) {
          const angle = (b * 2.399963) % (2 * Math.PI);
          dx = Math.cos(angle);
          dy = Math.sin(angle);
          distance = 1;
        }
        const push = (gap - distance) / 2 + 0.01;
        const ux = dx / distance;
        const uy = dy / distance;
        first.x -= ux * push;
        first.y -= uy * push;
        second.x += ux * push;
        second.y += uy * push;
        moved = true;
      }
    }
    if (!moved) break;
  }
  for (const marker of placed) {
    marker.x = Math.min(MAP_WIDTH - gap / 2, Math.max(gap / 2, marker.x));
    marker.y = Math.min(MAP_HEIGHT - gap / 2, Math.max(gap / 2, marker.y));
  }
  return placed;
}
