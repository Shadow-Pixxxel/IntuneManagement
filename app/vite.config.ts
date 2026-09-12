import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

const serverPort = process.env.INTUNE_SERVER_PORT ?? "8787";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  // Tauri expects a fixed port and fails if it is not available.
  clearScreen: false,
  server: {
    host: "0.0.0.0",
    port: 5173,
    strictPort: true,
    // In browser/dev mode the frontend talks to the axum sidecar via this proxy.
    // In the Tauri desktop app, native `invoke` is used instead and the proxy is unused.
    proxy: {
      "/api": {
        target: `http://127.0.0.1:${serverPort}`,
        changeOrigin: true,
      },
    },
  },
});
