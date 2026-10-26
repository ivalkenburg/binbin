import { defineConfig } from "@playwright/test";
import { fileURLToPath } from "node:url";
export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 60000,
  use: { baseURL: "http://localhost:8086", trace: "retain-on-failure" },
  webServer: {
    cwd: fileURLToPath(new URL("..", import.meta.url)),
    command: "cargo run -p binbin-server --example test_server",
    url: "http://localhost:8086/healthz",
    reuseExistingServer: false,
    timeout: 120000,
  },
  projects: [
    ...["light", "dark"].flatMap((colorScheme) => [
      {
        name: `desktop-${colorScheme}`,
        testIgnore: colorScheme === "light" ? [] : ["**/regressions.spec.js"],
        use: { viewport: { width: 1440, height: 1000 }, colorScheme },
      },
      {
        name: `tablet-${colorScheme}`,
        testIgnore: ["**/regressions.spec.js"],
        use: { viewport: { width: 768, height: 1024 }, colorScheme },
      },
      {
        name: `mobile-${colorScheme}`,
        testIgnore: ["**/regressions.spec.js"],
        use: { viewport: { width: 390, height: 844 }, colorScheme },
      },
      {
        name: `narrow-${colorScheme}`,
        testIgnore: ["**/regressions.spec.js"],
        use: { viewport: { width: 320, height: 740 }, colorScheme },
      },
    ]),
  ],
});
