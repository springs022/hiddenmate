import { defineConfig } from "vitest/config";

export default defineConfig({
  define: {
    FMRS_API_BASE_URL: JSON.stringify(""),
    FMRS_BASE_PATH: JSON.stringify("/"),
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./app/src/setupTests.ts"],
  },
});
