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
