import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    exclude: ["test-e2e/**", "node_modules/**", "dist/**", "src-tauri/target/**"],
    environment: "node",
  },
});
