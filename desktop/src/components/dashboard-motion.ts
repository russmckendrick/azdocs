import { useEffect, useRef, useState } from "react";

/** Interrupted updates continue from the currently displayed number. */
export function useDashboardNumber(value: number) {
  const [displayed, setDisplayed] = useState(value);
  const current = useRef(value);
  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    let frame = 0;
    const start = performance.now();
    const from = current.current;
    const finish = () => {
      cancelAnimationFrame(frame);
      current.current = value;
      setDisplayed(value);
    };
    const tick = (now: number) => {
      const progress = Math.min(1, (now - start) / 360);
      current.current = from + (value - from) * (1 - Math.pow(1 - progress, 3));
      setDisplayed(current.current);
      if (progress < 1) frame = requestAnimationFrame(tick);
    };
    const change = () => {
      if (preference.matches) finish();
    };
    preference.addEventListener("change", change);
    if (preference.matches) finish();
    else frame = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(frame);
      preference.removeEventListener("change", change);
    };
  }, [value]);
  return Math.round(displayed).toLocaleString();
}
