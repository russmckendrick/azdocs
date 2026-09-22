import { useId } from "react";
import { GRATICULE_PATH, LAND_PATH } from "./world-map";

/** Both map surfaces share the projection, shading and geographic grid. */
export function WorldMapBackdrop() {
  const landGradientId = useId();
  return (
    <g aria-hidden="true" className="world-map-backdrop">
      <defs>
        <linearGradient id={landGradientId} x1="0" y1="0" x2="0" y2="1">
          <stop className="world-map-land-top" offset="0%" />
          <stop className="world-map-land-bottom" offset="100%" />
        </linearGradient>
      </defs>
      <path className="world-map-grid" d={GRATICULE_PATH} />
      <path className="world-map-land" d={LAND_PATH} fill={`url(#${landGradientId})`} fillRule="evenodd" />
    </g>
  );
}
