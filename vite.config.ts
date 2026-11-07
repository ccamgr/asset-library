import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    allowedHosts: ["pve-dev.cow-agama.ts.net"],
    port: 1420,
    strictPort: true,
  },
});
