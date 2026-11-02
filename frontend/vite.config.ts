import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const baseDomain = process.env.VITE_BASE_DOMAIN ?? "guildos.localhost";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    port: 5173,
    // Todo subdomínio do domínio base (app., admin., {slug}.) chega neste dev server.
    allowedHosts: [baseDomain, `.${baseDomain}`],
    proxy: {
      // Sem changeOrigin: a API precisa do Host original para resolver o tenant.
      "/api": { target: process.env.VITE_API_PROXY ?? "http://127.0.0.1:8080", changeOrigin: false },
    },
  },
  test: {
    environment: "node",
  },
});
