import { test, expect } from "@playwright/test";
import { login } from "./support/session.js";
import { seed } from "./support/shares.js";

test("upload, publish, download ZIP, and revoke", async ({ page, browser }) => {
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await login(page);
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await page.locator("#files").setInputFiles([
    {
      name: "hello.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("hello world"),
    },
    {
      name: "notes.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("handover notes"),
    },
  ]);
  const title = `Browser share ${test.info().project.name}`;
  await page.getByLabel("Title", { exact: true }).fill(title);
  await page.getByLabel("Download allowance", { exact: true }).fill("2");
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await page
    .getByRole("button", { name: "Publish share", exact: true })
    .click();
  await expect(page.locator("#share-url")).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath("published-share.png"),
    fullPage: true,
  });
  const url = await page.locator("#share-url").inputValue();
  const context = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  const recipient = await context.newPage();
  await recipient.goto(url);
  await expect(recipient.getByRole("heading", { name: title })).toBeVisible();
  const downloading = recipient.waitForEvent("download");
  await recipient.getByRole("button", { name: /Download all as ZIP/ }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toMatch(/\.zip$/);
  expect(await download.failure()).toBeNull();
  await expect(
    recipient.getByRole("link", { name: /Retry this download/ }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Revoke and delete", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Confirm deletion", exact: true })
    .click();
  await recipient.reload();
  await expect(
    recipient.getByRole("heading", { name: "Share unavailable" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await context.close();
});
test("password hides metadata until unlocked", async ({ page, browser }) => {
  await login(page);
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await page.locator("#files").setInputFiles({
    name: "private.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("private"),
  });
  await page.getByLabel("Password optional").fill("private-share-password");
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await page
    .getByRole("button", { name: "Publish share", exact: true })
    .click();
  const url = await page.locator("#share-url").inputValue();
  const context = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  const recipient = await context.newPage();
  await recipient.goto(url);
  await expect(
    recipient.getByRole("heading", { name: "Password required" }),
  ).toBeVisible();
  await expect(recipient.getByText("private.txt", { exact: true })).toHaveCount(
    0,
  );
  await recipient
    .getByLabel("Share password", { exact: true })
    .fill("private-share-password");
  await recipient.getByRole("button", { name: "Open share" }).click();
  await expect(
    recipient.getByRole("heading", { name: "private.txt" }),
  ).toBeVisible();
  await context.close();
});

test("resume a private draft and verify its existing prefix", async ({
  page,
}) => {
  await login(page);
  const draft = await page.evaluate(async () => {
    const me = await (await fetch("/api/me")).json();
    const headers = {
      "content-type": "application/json",
      "x-binbin-csrf": me.csrf,
    };
    const draft = await (
      await fetch("/api/shares", {
        method: "POST",
        headers,
        body: JSON.stringify({
          title: "Resumable draft",
          files: [{ name: "resume.txt", size: 5 }],
          days: 7,
          limit: null,
          password: null,
        }),
      })
    ).json();
    const r = await fetch(`/api/files/${draft.files[0].id}?offset=0`, {
      method: "PATCH",
      headers: { "x-binbin-csrf": me.csrf },
      body: "he",
    });
    if (!r.ok) throw new Error("Cannot prepare draft");
    return draft;
  });
  await page.goto(`/#share/${draft.id}`);
  await expect(
    page.getByRole("heading", { name: "Resume this upload" }),
  ).toBeVisible();
  await page.locator("#files").setInputFiles({
    name: "resume.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("hello"),
  });
  await page
    .getByRole("button", { name: "Resume upload", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Publish share", exact: true })
    .click();
  await expect(page.locator("#share-url")).toBeVisible();
});

test("dashboard filters by status, sorts, and copies a share link", async ({
  page,
}) => {
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await login(page);
  const suffix = `dash${Date.now()}`;
  const published = await seed(page, { title: `Zulu ${suffix}` });
  await seed(page, { title: `Alpha ${suffix}`, partial: true });
  await page.reload();
  const rows = page.locator(".share-row").filter({ hasText: suffix });
  await expect(rows).toHaveCount(2);
  const chips = page.getByRole("group", { name: "Status" });
  await chips.getByRole("button", { name: /^Draft/ }).click();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText(`Alpha ${suffix}`);
  await chips.getByRole("button", { name: /^All shares/ }).click();
  await page.getByLabel("Sort shares").selectOption("title");
  await expect(rows.first()).toContainText(`Alpha ${suffix}`);
  await expect(
    rows.first().getByRole("button", { name: "Copy link" }),
  ).toHaveCount(0);
  await rows
    .filter({ hasText: `Zulu ${suffix}` })
    .getByRole("button", { name: "Copy link" })
    .click();
  await expect(page.locator("#notice")).toContainText("Link copied");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    published.url,
  );
});

test("expiry and allowance presets fill the new share form", async ({
  page,
}) => {
  await login(page);
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  const days = page.getByLabel("Expires after (days)", { exact: true });
  const allowance = page.getByLabel("Download allowance", { exact: true });
  await page
    .getByRole("group", { name: "days presets" })
    .getByRole("button", { name: "30", exact: true })
    .click();
  await expect(days).toHaveValue("30");
  await page
    .getByRole("group", { name: "days presets" })
    .getByRole("button", { name: "No deadline" })
    .click();
  await expect(days).toHaveValue("");
  await page
    .getByRole("group", { name: "limit presets" })
    .getByRole("button", { name: "5", exact: true })
    .click();
  await expect(allowance).toHaveValue("5");
  await page.getByRole("link", { name: "Shares", exact: true }).click();
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await expect(days).toHaveValue("");
  await expect(allowance).toHaveValue("5");
});
