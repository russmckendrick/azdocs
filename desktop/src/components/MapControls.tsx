import { Maximize2, ZoomIn, ZoomOut } from "lucide-react";
import { useLabels } from "../labels";

/** The joined zoom ribbon both maps share, tucked into the frame's corner. */
export function MapControls({
  onZoomIn,
  onZoomOut,
  onReset,
  canZoomIn,
  canZoomOut,
}: {
  onZoomIn: () => void;
  onZoomOut: () => void;
  onReset: () => void;
  canZoomIn: boolean;
  canZoomOut: boolean;
}) {
  const words = useLabels().desktop.regions;
  return (
    <div className="map-controls" role="group" aria-label={words.map_controls_aria}>
      <button type="button" onClick={onZoomIn} disabled={!canZoomIn} aria-label={words.zoom_in} title={words.zoom_in}>
        <ZoomIn size={15} />
      </button>
      <button type="button" onClick={onZoomOut} disabled={!canZoomOut} aria-label={words.zoom_out} title={words.zoom_out}>
        <ZoomOut size={15} />
      </button>
      <button type="button" onClick={onReset} disabled={!canZoomOut} aria-label={words.zoom_reset} title={words.zoom_reset}>
        <Maximize2 size={15} />
      </button>
    </div>
  );
}
