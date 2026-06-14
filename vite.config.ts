import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    strictPort: true,
    port: 3000,
    watch: {
      ignored: ["**/src-tauri/**", "**/tests/asset/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
});
