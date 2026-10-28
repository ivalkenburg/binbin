import { test, expect } from "@playwright/test";
import { login } from "./support/session.js";
import { seed } from "./support/shares.js";
import { browserFiles } from "./support/downloads.js";

// Full Chromium provides the native PDF viewer omitted by headless shell.
test.use({ channel: "chromium" });
test("browser opening renders images, audio and PDF and keeps HTML inert", async ({
  page,
}) => {
  await login(page);
  const files = browserFiles();
  await page.goto("/#new");
  await page.locator("#files").setInputFiles(
    files.map((file) => ({
      ...file,
      mimeType: "application/octet-stream",
    })),
  );
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await page
    .getByRole("button", { name: "Publish share", exact: true })
    .click();
  await expect(page.locator("#share-url")).toBeVisible();
  await page.goto(await page.locator("#share-url").inputValue());
  for (const { name } of files) {
    const opened = page.waitForEvent("popup");
    await page
      .getByRole("button", {
        name: `Open ${name} in browser (new tab)`,
        exact: true,
      })
      .click();
    const file = await opened;
    try {
      await file.waitForURL(/\?inline=true$/);
      if (name.endsWith(".png"))
        await expect
          .poll(() => file.locator("img").evaluate((img) => img.naturalWidth))
          .toBe(1);
      else if (name.endsWith(".wav"))
        await expect
          .poll(() =>
            file.locator("audio, video").evaluate((media) => media.readyState),
          )
          .toBeGreaterThanOrEqual(2);
      else if (name.endsWith(".pdf")) {
        await expect
          .poll(() =>
            file
              .frames()
              .map((frame) => frame.url())
              .join(" "),
          )
          .toContain("chrome-extension://");
        await file.screenshot({
          path: test.info().outputPath("opened-pdf.png"),
        });
      } else {
        await expect(file.locator("body")).toHaveText(
          "<script>window.executed = true</script>",
        );
        expect(await file.evaluate(() => window.executed)).toBeUndefined();
      }
    } finally {
      await file.close();
    }
  }
});

test("file names open in the browser and downloads reuse the same allowance", async ({
  page,
  context,
}) => {
  await login(page);
  const share = await seed(page, { password: "browser-open-password" });
  const recipient = await context.newPage();
  try {
    await recipient.goto(share.url);
    await recipient
      .getByLabel("Share password", { exact: true })
      .fill("browser-open-password");
    await recipient.getByRole("button", { name: "Open share" }).click();
    let issued = 0;
    recipient.on("request", (request) => {
      if (request.url().endsWith("/transfers")) issued++;
    });
    const opened = recipient.waitForEvent("popup");
    await recipient
      .getByRole("button", {
        name: "Open handover.txt in browser (new tab)",
        exact: true,
      })
      .click();
    const file = await opened;
    try {
      await file.waitForURL(/\/d\/[^?]+\?inline=true$/);
      await expect(file.locator("body")).toHaveText("hello");
      expect(await file.evaluate(() => window.opener)).toBeNull();
      const href = await recipient
        .getByRole("link", { name: "Retry this download" })
        .getAttribute("href");
      expect(new URL(file.url()).pathname).toBe(href);
      const saving = recipient.waitForEvent("download");
      await recipient
        .getByRole("button", { name: "Download", exact: true })
        .click();
      expect((await saving).suggestedFilename()).toBe("handover.txt");
      expect(issued).toBe(1);
      const current = await (
        await page.request.get(`/api/shares/${share.id}`)
      ).json();
      expect(current.used).toBe(1);
      await expect(recipient.locator(".transfer-feedback")).toHaveCount(1);
      await recipient
        .getByRole("button", { name: "Dismiss download message" })
        .click();
      await expect(recipient.locator("#retry-links")).toBeEmpty();
    } finally {
      await file.close();
    }
  } finally {
    await recipient.close();
  }
});

test("management file names open without using a public download", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await page.goto(`/#share/${share.id}`);
  const opened = page.waitForEvent("popup");
  await page
    .getByRole("button", {
      name: "Open handover.txt in browser (new tab)",
      exact: true,
    })
    .click();
  const file = await opened;
  try {
    await file.waitForURL(/\?inline=true$/);
    await expect(file.locator("body")).toHaveText("hello");
    const saving = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    await saving;
    const current = await (
      await page.request.get(`/api/shares/${share.id}`)
    ).json();
    expect(current.used).toBe(0);
  } finally {
    await file.close();
  }
});

test("blocked tabs explain recovery without issuing a transfer", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await page.goto(share.url);
  await page.evaluate(() => {
    window.open = () => null;
  });
  let issued = 0;
  page.on("request", (request) => {
    if (request.url().endsWith("/transfers")) issued++;
  });
  await page
    .getByRole("button", {
      name: "Open handover.txt in browser (new tab)",
      exact: true,
    })
    .click();
  await expect(page.locator("#notice")).toContainText("Allow a new tab");
  expect(issued).toBe(0);
});

test("concurrent Download and Retry actions save one attachment", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await page.goto(share.url);
  const button = page.getByRole("button", { name: "Download", exact: true });
  const first = page.waitForEvent("download");
  await button.click();
  await first;
  let release, entered;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  const checking = new Promise((resolve) => {
    entered = resolve;
  });
  let heads = 0;
  await page.route("**/d/*", async (route) => {
    if (route.request().method() === "HEAD") {
      heads++;
      entered();
      await held;
    }
    await route.continue();
  });
  let saves = 0;
  page.on("download", () => saves++);
  await button.click();
  await checking;
  await page.getByRole("link", { name: "Retry this download" }).click();
  const saving = page.waitForEvent("download");
  release();
  await saving;
  await expect(button).toHaveAttribute("aria-busy", "false");
  expect(heads).toBe(1);
  expect(saves).toBe(1);
});
