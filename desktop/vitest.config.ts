import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Deliberately NOT reusing vite.config.ts: nothing here needs the dev server
// or the asset sync. Two projects: pure functions over the DTOs run in node;
// component tests (`*.test.tsx`) run in jsdom with Testing Library and mock
// the `api` module, so they exercise the same components the app renders
// without Tauri. `__BROWSER_PREVIEW__` is on so `api.ts` compiles its
// preview branches; the mocks decide what each call returns.
export default defineConfig({
  define: {
    __BROWSER_PREVIEW__: JSON.stringify(true),
  },
  test: {
    // Reset every vi.fn() between tests to the implementation it was
    // created with, so one test's `mockResolvedValue` never leaks.
    mockReset: true,
    restoreMocks: true,
    projects: [
      {
        extends: true,
        test: {
          name: "unit",
          include: ["src/**/*.test.ts"],
          environment: "node",
        },
      },
      {
        extends: true,
        plugins: [react()],
        test: {
          name: "components",
          include: ["src/**/*.test.tsx"],
          environment: "jsdom",
          // jsdom's default about:blank origin has no localStorage; the
          // preferences module needs a real one.
          environmentOptions: { jsdom: { url: "http://localhost/" } },
          setupFiles: ["src/test-setup.ts"],
        },
      },
    ],
  },
});
