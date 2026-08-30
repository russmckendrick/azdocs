import { defineConfig } from "vitest/config";

// Deliberately NOT reusing vite.config.ts. That config sets
// `publicDir: "../data"`, so inheriting it mounted the whole vendored Azure
// icon catalogue as a static root on every test run. The suite is pure
// functions over the topology DTO and needs no DOM and no assets.
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    restoreMocks: true,
  },
});
