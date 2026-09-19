import { defineConfig } from "vitest/config";

// Deliberately NOT reusing vite.config.ts: the suite is pure functions over
// the DTOs and needs no DOM, no dev server and no assets.
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    restoreMocks: true,
  },
});
