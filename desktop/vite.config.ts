import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Ship the complete vendored Microsoft Azure icon catalogue with the app.
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
