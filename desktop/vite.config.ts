import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri sets TAURI_ENV_* for the commands it drives (see beforeBuildCommand in
// tauri.conf.json). The mock estate and the browser-preview topology builder are
// ~800 lines that can never run inside the app, where `isTauri` is always true;
// gating on this lets Rollup drop them from the shipped bundle while a plain
// `vite build` still produces a working browser demo.
const buildingForTauri = Boolean(process.env.TAURI_ENV_PLATFORM);

export default defineConfig({
  plugins: [react()],
  define: {
    __BROWSER_PREVIEW__: JSON.stringify(!buildingForTauri),
  },
  // Ship the complete vendored Microsoft Azure icon catalogue with the app.
  // This also serves the IBM Plex Sans/Mono faces styles.css loads from /fonts/.
  publicDir: "../data",
  clearScreen: false,
  server: {
    strictPort: true,
    host: "127.0.0.1",
    port: 1420,
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    // Cytoscape.js is isolated behind the lazy-loaded Relationships route.
    chunkSizeWarningLimit: 600,
  },
});
