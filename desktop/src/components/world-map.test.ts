import { describe, expect, it } from "vitest";
import { spreadMarkers } from "./world-map";

describe("spreadMarkers", () => {
  it("separates neighbouring markers to at least the gap", () => {
    const placed = spreadMarkers(
      [
        { x: 200, y: 60 },
        { x: 203, y: 58 },
        { x: 201, y: 62 },
      ],
      14,
    );
    for (let a = 0; a < placed.length; a += 1) {
      for (let b = a + 1; b < placed.length; b += 1) {
        expect(Math.hypot(placed[a].x - placed[b].x, placed[a].y - placed[b].y)).toBeGreaterThanOrEqual(13.9);
      }
    }
  });

  it("keeps each marker's true location as its anchor", () => {
    const [first, second] = spreadMarkers(
      [
        { x: 100, y: 50 },
        { x: 100, y: 50 },
      ],
      14,
    );
    expect([first.anchorX, first.anchorY, second.anchorX, second.anchorY]).toEqual([100, 50, 100, 50]);
    expect(Math.hypot(first.x - second.x, first.y - second.y)).toBeGreaterThanOrEqual(13.9);
  });

  it("leaves isolated markers exactly where they are", () => {
    const [marker] = spreadMarkers([{ x: 80, y: 40 }], 14);
    expect([marker.x, marker.y]).toEqual([80, 40]);
  });
});
