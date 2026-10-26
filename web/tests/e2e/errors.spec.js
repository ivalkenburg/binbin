import { test, expect } from "@playwright/test";
import { login } from "./support/session.js";
import { seed } from "./support/shares.js";

test("failed page requests remain readable and contained", async ({ page }) => {
  await login(page);
  const message =
    "Cannot load API tokens. Check your connection and try again. Reference: " +
    "unbroken-reference-".repeat(5);
  await page.route("**/api/tokens", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ error: message }),
    }),
  );
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  const alert = page.locator("main").getByRole("alert");
  await expect(alert).toHaveText(message);
  const geometry = await alert.locator("p").evaluate((element) => {
    const text = element.getBoundingClientRect();
    const parent = element.closest('[role="alert"]').getBoundingClientRect();
    return {
      width: text.width,
      left: text.left,
      right: text.right,
      parentLeft: parent.left,
      parentRight: parent.right,
      document: document.documentElement.scrollWidth,
      viewport: document.documentElement.clientWidth,
    };
  });
  expect(geometry.width).toBeGreaterThan(200);
  expect(geometry.left).toBeGreaterThanOrEqual(geometry.parentLeft);
  expect(geometry.right).toBeLessThanOrEqual(geometry.parentRight);
  expect(geometry.document).toBeLessThanOrEqual(geometry.viewport + 1);
  await page.screenshot({
    path: test.info().outputPath("request-error.png"),
    fullPage: true,
    animations: "disabled",
  });
});

test("temporary recipient failures offer a working retry", async ({ page }) => {
  await login(page);
  const share = await seed(page, { title: "Retry after a temporary failure" });
  await page.route(
    `**/api/public/${share.id}`,
    (route) =>
      route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ error: "Temporarily unavailable" }),
      }),
    { times: 1 },
  );
  await page.goto(share.url);
  await expect(
    page.getByRole("heading", { name: "Couldn’t load this share" }),
  ).toBeVisible();
  await expect(
    page.getByText("Check your connection and try again.", { exact: false }),
  ).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath("recipient-request-error.png"),
    fullPage: true,
    animations: "disabled",
  });
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: share.title, exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Download", exact: true }),
  ).toBeVisible();
});
