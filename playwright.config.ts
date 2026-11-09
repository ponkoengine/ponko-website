import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  timeout: 70_000,
  fullyParallel: false,
  workers: 1,
  use: {
    baseURL: "http://127.0.0.1:4321",
    viewport: { width: 1440, height: 1000 },
    launchOptions: {
      args: [
        "--enable-unsafe-webgpu",
        process.platform === "darwin"
          ? "--use-angle=metal"
          : "--use-angle=swiftshader",
      ],
    },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npm run preview -- --port 4321 --ignore-lock",
    url: "http://127.0.0.1:4321",
    reuseExistingServer: !process.env.CI,
    env: { ASTRO_TELEMETRY_DISABLED: "1" },
  },
});
