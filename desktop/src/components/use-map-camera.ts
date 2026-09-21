import { useCallback, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";

/**
 * A map camera: the land scales about the frame while markers and labels are
 * re-projected through it, so they keep their pixel size at any zoom. Both
 * maps share this; the Overview's works in viewBox units and the Regions
 * page's in pixels, which is why pointer deltas go through `unitsPerPixel`.
 */
export interface MapCamera {
  k: number;
  tx: number;
  ty: number;
}

export const MAP_ZOOM_MAX = 8;
export const MAP_ZOOM_STEP = 1.6;
/** Pointer travel before a press counts as a drag rather than a click. */
const DRAG_THRESHOLD = 3;

export const IDENTITY_CAMERA: MapCamera = { k: 1, tx: 0, ty: 0 };

/** The map can never be dragged clear of its frame. */
export function clampCamera(camera: MapCamera, width: number, height: number): MapCamera {
  const k = Math.min(MAP_ZOOM_MAX, Math.max(1, camera.k));
  return {
    k,
    tx: Math.min(0, Math.max(width - width * k, camera.tx)),
    ty: Math.min(0, Math.max(height - height * k, camera.ty)),
  };
}

/** Zoom by `factor` about the frame point (cx, cy). */
export function zoomCamera(
  camera: MapCamera,
  factor: number,
  cx: number,
  cy: number,
  width: number,
  height: number,
): MapCamera {
  const k = Math.min(MAP_ZOOM_MAX, Math.max(1, camera.k * factor));
  const ratio = k / camera.k;
  return clampCamera(
    { k, tx: cx - (cx - camera.tx) * ratio, ty: cy - (cy - camera.ty) * ratio },
    width,
    height,
  );
}

export function panCamera(camera: MapCamera, dx: number, dy: number, width: number, height: number) {
  return clampCamera({ ...camera, tx: camera.tx + dx, ty: camera.ty + dy }, width, height);
}

export function applyCamera(camera: MapCamera, x: number, y: number): [number, number] {
  return [x * camera.k + camera.tx, y * camera.k + camera.ty];
}

export function useMapCamera(width: number, height: number, unitsPerPixel: () => number = () => 1) {
  const [camera, setCamera] = useState<MapCamera>(IDENTITY_CAMERA);
  const drag = useRef<{ pointerId: number; x: number; y: number; moved: boolean } | null>(null);
  const [panning, setPanning] = useState(false);

  const zoomIn = useCallback(
    () => setCamera((current) => zoomCamera(current, MAP_ZOOM_STEP, width / 2, height / 2, width, height)),
    [width, height],
  );
  const zoomOut = useCallback(
    () => setCamera((current) => zoomCamera(current, 1 / MAP_ZOOM_STEP, width / 2, height / 2, width, height)),
    [width, height],
  );
  const reset = useCallback(() => setCamera(IDENTITY_CAMERA), []);

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<SVGSVGElement>) => {
      if (event.button !== 0 || camera.k === 1) return;
      drag.current = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
      event.currentTarget.setPointerCapture(event.pointerId);
      setPanning(true);
    },
    [camera.k],
  );
  const onPointerMove = useCallback(
    (event: ReactPointerEvent<SVGSVGElement>) => {
      const state = drag.current;
      if (!state || state.pointerId !== event.pointerId) return;
      const dx = event.clientX - state.x;
      const dy = event.clientY - state.y;
      if (!state.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
      state.moved = true;
      state.x = event.clientX;
      state.y = event.clientY;
      const units = unitsPerPixel();
      setCamera((current) => panCamera(current, dx * units, dy * units, width, height));
    },
    [width, height, unitsPerPixel],
  );
  const onPointerUp = useCallback((event: ReactPointerEvent<SVGSVGElement>) => {
    const state = drag.current;
    if (!state || state.pointerId !== event.pointerId) return;
    // Cleared after the click that follows pointerup has had its look.
    window.setTimeout(() => {
      drag.current = null;
    }, 0);
    setPanning(false);
  }, []);

  /** True while the press that just ended was a drag, so a marker ignores it. */
  const consumedByDrag = useCallback(() => drag.current?.moved === true, []);

  return {
    camera,
    panning,
    zoomIn,
    zoomOut,
    reset,
    consumedByDrag,
    canZoomIn: camera.k < MAP_ZOOM_MAX,
    canZoomOut: camera.k > 1,
    pointerHandlers: { onPointerDown, onPointerMove, onPointerUp, onPointerCancel: onPointerUp },
  };
}
