import { test, expect } from "@playwright/test";
import themeTokens from "./support/theme.json" with { type: "json" };

// Exercise actual routes and requests: recipient contexts must inherit the
// project viewport, or a "mobile" test quietly checks desktop download pages.
async function checkLayout(page, name, { screenshot = true } = {}) {
  await expect(page.locator("h1")).toBeVisible();
  if (!name.startsWith("login")) await expect(page).not.toHaveTitle(/^Sign in/);
  const geometry = await page.evaluate(() => {
    const width = document.documentElement.clientWidth;
    const overflowing = [
      ...document.querySelectorAll("main *, header, footer, #notice"),
    ]
      .filter((el) => {
        const rect = el.getBoundingClientRect();
        const css = getComputedStyle(el);
        if (
          !rect.width ||
          !rect.height ||
          css.position === "absolute" ||
          (css.position === "fixed" && !el.matches("#notice.visible"))
        )
          return false;
        return rect.left < -1 || rect.right > width + 1;
      })
      .map((el) => ({
        tag: el.tagName,
        class: el.className,
        id: el.id,
        width: Math.round(el.getBoundingClientRect().width),
      }));
    const containers = [
      "main",
      ".work-area",
      ".share-options",
      ".recipient-share",
      ".transfer-feedback",
      "#notice.visible",
    ]
      .flatMap((selector) => [...document.querySelectorAll(selector)])
      .filter((el) => el.clientWidth && el.scrollWidth > el.clientWidth + 1)
      .map((el) => ({
        class: el.className,
        scroll: el.scrollWidth,
        width: el.clientWidth,
      }));
    return {
      viewport: width,
      document: document.documentElement.scrollWidth,
      overflowing,
      containers,
    };
  });
  // A visible vertical scrollbar can reduce clientWidth on Linux/Windows.
  expect(geometry.document, name).toBeLessThanOrEqual(geometry.viewport + 1);
  expect(geometry.overflowing, name).toEqual([]);
  expect(geometry.containers, name).toEqual([]);
  const theme = await page.evaluate(() => {
    const root = document.documentElement;
    const css = getComputedStyle(root);
    return {
      dark: root.classList.contains("dark"),
      tokens: Object.fromEntries(
        [...css]
          .filter((key) => key.startsWith("--"))
          .map((key) => [key.slice(2), css.getPropertyValue(key).trim()]),
      ),
    };
  });
  for (const [token, value] of Object.entries(
    themeTokens[theme.dark ? "dark" : "light"],
  )) {
    // Production CSS may serialize lightness as a percentage.
    const channels = (color) =>
      color
        .match(/[\d.]+%?/g)
        .map(
          (part) =>
            Math.round(
              (parseFloat(part) / (part.endsWith("%") ? 100 : 1)) * 1e6,
            ) / 1e6,
        );
    expect(channels(theme.tokens[token]), token).toEqual(channels(value));
  }
  if (screenshot)
    await page.screenshot({
      path: test.info().outputPath(`${name}.png`),
      fullPage: true,
      animations: "disabled",
    });
}
import { login } from "./support/session.js";
import { seed } from "./support/shares.js";

test("every page and transient state fits its viewport", async ({
  page,
  browser,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await checkLayout(page, "login");
  await page.getByLabel("Username", { exact: true }).fill("admin");
  await page.getByLabel("Password", { exact: true }).fill("incorrect-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("#login-error")).toContainText("Invalid");
  await checkLayout(page, "login-error");
  await login(page);
  await checkLayout(page, "shares");
  const longName = "handover-" + "long-filename-".repeat(12) + ".txt";
  const share = await seed(page, {
    filename: longName,
    title: "Quarterly handover · " + test.info().project.name,
  });
  await page.reload();
  await expect(page.locator(".share-row")).not.toHaveCount(0);
  await checkLayout(page, "shares-populated");
  await page
    .getByRole("searchbox", { name: "Search shares" })
    .fill("no-result-for-this-search-12345");
  await expect(page.locator("#no-results")).toBeVisible();
  await checkLayout(page, "shares-no-results");
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("heading", { name: "New share", exact: true }),
  ).toBeVisible();
  await checkLayout(page, "new-share");
  await page.locator("#files").setInputFiles({
    name: longName,
    mimeType: "text/plain",
    buffer: Buffer.from("hello"),
  });
  await checkLayout(page, "files-selected");
  let release;
  const held = new Promise((resolve) => (release = resolve));
  await page.route("**/api/files/*?offset=*", async (route) => {
    await held;
    await route.continue();
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await expect(
    page.locator(".queued-file[data-state=uploading]"),
  ).toBeVisible();
  await checkLayout(page, "upload-progress");
  const committed = page.waitForResponse(
    (response) =>
      response.url().includes("/api/files/") &&
      response.request().method() === "PATCH",
  );
  release();
  await committed;
  await page.unroute("**/api/files/*?offset=*");
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeVisible();
  await checkLayout(page, "draft-ready");
  const draft = await seed(page, {
    partial: true,
    title: "Interrupted upload",
  });
  await page.goto(`/#share/${draft.id}`);
  await expect(
    page.getByRole("heading", { name: "Resume this upload" }),
  ).toBeVisible();
  await checkLayout(page, "draft-resume");
  await page.goto(`/#share/${share.id}`);
  await expect(page.locator("#share-url")).toBeVisible();
  await checkLayout(page, "share-details");
  await page
    .getByRole("button", { name: "Edit settings", exact: true })
    .click();
  await checkLayout(page, "share-edit");
  await expect(page.locator("#share-overview")).not.toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Edit share", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Cancel changes", exact: true })
    .click();
  await expect(page.locator("#share-overview")).toBeVisible();
  await page
    .getByRole("button", { name: "Edit settings", exact: true })
    .click();
  await page
    .locator('#edit-share input[name="title"]')
    .fill(`${share.title} · reviewed`);
  await page.getByRole("button", { name: "Save changes", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: `${share.title} · reviewed`,
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.locator("#share-overview")).toBeVisible();
  await page
    .getByRole("button", { name: "Revoke and delete", exact: true })
    .click();
  await expect(page.locator("#notice")).not.toBeVisible();
  await expect(page.locator("#delete-confirm")).toContainText(
    "There is no undo.",
  );
  await checkLayout(page, "delete-confirmation");
  await page.getByRole("button", { name: "Keep share" }).click();
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("heading", { name: "API tokens" })).toBeVisible();
  await checkLayout(page, "settings");
  await page
    .getByLabel("Token name")
    .fill("Long device name " + "workstation ".repeat(6));
  await page.getByRole("button", { name: "Create token", exact: true }).click();
  await expect(page.locator("#token-value")).toBeVisible();
  await checkLayout(page, "token-reveal");
  await page
    .getByRole("button", { name: "I’ve saved it", exact: true })
    .click();
  let failRefresh = true;
  const refreshRoute = (route) => {
    if (route.request().method() === "GET" && failRefresh) {
      failRefresh = false;
      return route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({
          error:
            "Temporary refresh failure. Try again to update these details.",
        }),
      });
    }
    return route.continue();
  };
  await page.route("**/api/tokens", refreshRoute);
  await page
    .locator(".token-row")
    .last()
    .getByRole("button", { name: "Revoke", exact: true })
    .click();
  await expect(
    page.getByText("Couldn’t refresh. Details shown may be out of date."),
  ).toBeVisible();
  await checkLayout(page, "refresh-error");
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(
    page.getByText("Couldn’t refresh. Details shown may be out of date."),
  ).not.toBeVisible();
  await page.unroute("**/api/tokens", refreshRoute);
  await page.getByRole("link", { name: "Administration", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Accounts", exact: true }),
  ).toBeVisible();
  await checkLayout(page, "administration");
  const username = "review_" + test.info().project.name.replaceAll("-", "_");
  await page.getByLabel("Username", { exact: true }).fill(username);
  await page
    .getByLabel("Initial password", { exact: true })
    .fill("review-account-password");
  await page
    .getByRole("button", { name: "Create account", exact: true })
    .click();
  const row = page.locator(".user-row").filter({ hasText: username });
  await expect(row).toBeVisible();
  await row
    .getByRole("button", { name: "Reset password", exact: true })
    .click();
  await checkLayout(page, "password-reset");
  await row.getByRole("button", { name: "Disable", exact: true }).click();
  await expect(page.locator("#notice")).toContainText("Account updated");
  await checkLayout(page, "notification");
  const toast = await page.locator("#notice").boundingBox();
  expect(toast.width).toBeLessThanOrEqual(360);
  await page.getByRole("button", { name: "Dismiss notification" }).click();
  await expect(page.locator("#notice")).not.toBeVisible();
  await page
    .locator(".user-row")
    .filter({ hasText: username })
    .getByRole("button", { name: "Enable", exact: true })
    .click();
  const uploaderContext = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  const uploader = await uploaderContext.newPage();
  uploader.on("pageerror", (error) => errors.push(error.message));
  await uploader.goto("/");
  await uploader.getByLabel("Username", { exact: true }).fill(username);
  await uploader
    .getByLabel("Password", { exact: true })
    .fill("review-account-password");
  await uploader.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    uploader.getByRole("heading", { name: "No shares yet." }),
  ).toBeVisible();
  await expect(
    uploader.getByRole("link", { name: "Administration", exact: true }),
  ).toHaveCount(0);
  await checkLayout(uploader, "uploader-empty-shares");
  await uploader.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    uploader.getByText("No API tokens yet.", { exact: true }),
  ).toBeVisible();
  await checkLayout(uploader, "uploader-empty-settings");
  await uploaderContext.close();
  const context = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  const recipient = await context.newPage();
  recipient.on("pageerror", (error) => errors.push(error.message));
  await recipient.goto(share.url);
  await expect(
    recipient.getByRole("button", { name: "Download", exact: true }),
  ).toBeVisible();
  await checkLayout(recipient, "public-share");
  let completed = recipient.waitForEvent("download");
  await recipient
    .getByRole("button", { name: "Download", exact: true })
    .click();
  expect(await (await completed).failure()).toBeNull();
  await expect(
    recipient.getByRole("link", { name: "Retry this download", exact: true }),
  ).toBeVisible();
  await expect(recipient.locator("#notice")).not.toBeVisible();
  await checkLayout(recipient, "download-started");
  const feedback = await recipient.locator(".transfer-feedback").boundingBox();
  const panel = await recipient.locator(".recipient-share").boundingBox();
  expect(feedback.width).toBeLessThan(panel.width);
  expect(feedback.height).toBeLessThan(160);
  completed = recipient.waitForEvent("download");
  await recipient
    .getByRole("button", { name: "Download", exact: true })
    .click();
  await (await completed).failure();
  await expect(recipient.locator(".transfer-feedback")).toHaveCount(1);
  await recipient
    .getByRole("button", { name: "Dismiss download message" })
    .click();
  await expect(recipient.locator(".transfer-feedback")).toHaveCount(0);
  const limitedShare = await seed(page, {
    title: "Single download retry",
    limit: 1,
  });
  await recipient.goto(limitedShare.url);
  completed = recipient.waitForEvent("download");
  await recipient
    .getByRole("button", { name: "Download", exact: true })
    .click();
  await completed;
  await recipient.reload();
  await expect(
    recipient.getByRole("heading", { name: "Resume your download" }),
  ).toBeVisible();
  await checkLayout(recipient, "exhausted-share-retry");
  completed = recipient.waitForEvent("download");
  await recipient
    .getByRole("link", { name: "Retry download", exact: true })
    .click();
  await completed;
  const protectedShare = await seed(page, {
    title: "Protected files",
    password: "private-review-password",
  });
  await recipient.goto(protectedShare.url);
  await expect(
    recipient.getByRole("heading", { name: "Password required" }),
  ).toBeVisible();
  await checkLayout(recipient, "password-challenge");
  await recipient
    .getByLabel("Share password", { exact: true })
    .fill("incorrect-password");
  await recipient
    .getByRole("button", { name: "Open share", exact: true })
    .click();
  await expect(recipient.locator("#notice")).toContainText(
    "Incorrect password",
  );
  await checkLayout(recipient, "password-error");
  await recipient
    .getByLabel("Share password", { exact: true })
    .fill("private-review-password");
  await recipient
    .getByRole("button", { name: "Open share", exact: true })
    .click();
  await expect(
    recipient.getByRole("heading", { name: "Protected files" }),
  ).toBeVisible();
  await checkLayout(recipient, "password-unlocked");
  await recipient.goto("/s/nonexistent-share");
  await expect(
    recipient.getByRole("heading", { name: "Share unavailable" }),
  ).toBeVisible();
  await checkLayout(recipient, "unavailable-share");
  await page.goto(`/#share/${share.id}`);
  await expect(
    page.getByRole("button", { name: "Revoke and delete", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Revoke and delete", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Confirm deletion", exact: true })
    .click();
  await expect(
    page.getByText("File contents have been removed.", { exact: true }),
  ).toBeVisible();
  await checkLayout(page, "removed-share");
  expect(errors).toEqual([]);
  await context.close();
});

test("theme selection cycles one icon and persists on navigation", async ({
  page,
}) => {
  await page.goto("/");
  const theme = page.locator("#theme");
  await expect(page.getByRole("button", { name: /^Color theme:/ })).toHaveCount(
    1,
  );
  await expect(theme).toHaveAccessibleName(
    "Color theme: System. Switch to Light.",
  );
  await expect(theme.locator(".lucide-monitor")).toBeVisible();
  await theme.click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(theme.locator(".lucide-sun")).toBeVisible();
  await theme.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(theme).toHaveAccessibleName(
    "Color theme: Dark. Switch to System.",
  );
  await expect(theme.locator(".lucide-moon")).toBeVisible();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(theme.locator(".lucide-moon")).toBeVisible();
  await theme.click();
  await expect(page.locator("html")).not.toHaveAttribute("data-theme");
  await expect(theme.locator(".lucide-monitor")).toBeVisible();
  await theme.click();
  await login(page);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await theme.click();
  await theme.click();
  await expect(page.locator("html")).not.toHaveAttribute("data-theme");
  await expect(theme).toHaveAccessibleName(
    "Color theme: System. Switch to Light.",
  );
});
