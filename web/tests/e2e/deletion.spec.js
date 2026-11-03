import { test, expect } from "@playwright/test";
import { login } from "./support/session.js";
import { seed } from "./support/shares.js";

async function screenshot(page, name) {
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: test.info().outputPath(`${name}.png`),
    fullPage: true,
  });
}

test("delete-after-download settings, management exemption, ZIP delivery, and live removal", async ({
  page,
  browser,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await login(page);
  await page.goto("/#new");
  const policy = page.getByRole("checkbox", {
    name: "Delete after download",
    exact: true,
  });
  await expect(policy).not.toBeChecked();
  await policy.check();
  await page.getByRole("link", { name: "Shares", exact: true }).click();
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await expect(policy).toBeChecked();
  await expect(
    page.getByLabel("Download allowance", { exact: true }),
  ).toHaveCount(0);
  await page.locator("#files").setInputFiles([
    {
      name: "handover.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("hello"),
    },
    { name: "notes.txt", mimeType: "text/plain", buffer: Buffer.from("notes") },
  ]);
  const title = `One-time handover ${test.info().project.name}`;
  await page.getByLabel("Title", { exact: true }).fill(title);
  await screenshot(page, "delete-creation");
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await page
    .getByRole("button", { name: "Edit settings", exact: true })
    .click();
  await policy.uncheck();
  await expect(page.getByLabel("Total download allowance")).toBeVisible();
  await policy.check();
  await page.getByRole("button", { name: "Save changes", exact: true }).click();
  await page
    .getByRole("button", { name: "Publish share", exact: true })
    .click();
  await expect(page.locator("#share-url")).toBeVisible();
  const url = await page.locator("#share-url").inputValue();
  await screenshot(page, "delete-owner");
  const ownerDownload = page.waitForEvent("download");
  await page.locator('[data-action="download"][data-resource="zip"]').click();
  expect(await (await ownerDownload).failure()).toBeNull();
  await expect(page.locator("#share-url")).toBeVisible();
  await page
    .getByRole("button", { name: "Edit settings", exact: true })
    .click();
  await expect(policy).toHaveCount(0);
  await expect(
    page.getByText("Delete after download: enabled. Locked after publication."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Cancel changes" }).click();
  const context = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  try {
    const recipient = await context.newPage();
    recipient.on("pageerror", (error) => errors.push(error.message));
    await recipient.goto(url);
    await expect(
      recipient.getByRole("button", { name: "Download ZIP once" }),
    ).toBeVisible();
    await expect(recipient.locator(".file-open-link")).toHaveCount(0);
    await expect(
      recipient.locator('[data-action="public-download"]'),
    ).toHaveCount(1);
    await screenshot(recipient, "delete-recipient");
    const downloading = recipient.waitForEvent("download");
    await recipient.getByRole("button", { name: "Download ZIP once" }).click();
    const download = await downloading;
    expect(download.suggestedFilename()).toMatch(/\.zip$/);
    expect(await download.failure()).toBeNull();
    await expect(
      recipient.getByRole("heading", {
        name: "Deleted after download",
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Your shares" }),
    ).toBeVisible();
    await expect(
      page.locator(".share-row").filter({ hasText: title }),
    ).toHaveCount(0);
    await recipient.reload();
    await expect(
      recipient.getByRole("heading", { name: "Share unavailable" }),
    ).toBeVisible();
    await expect(
      recipient.getByText("handover.txt", { exact: true }),
    ).toHaveCount(0);
    expect(errors).toEqual([]);
  } finally {
    await context.close();
  }
});

test("draft settings stay in sync while an upload response is pending", async ({
  page,
}) => {
  test.skip(
    test.info().project.name !== "desktop-light",
    "One concurrency regression is sufficient across themes.",
  );
  await login(page);
  await page.goto("/#new");
  let entered;
  const started = new Promise((resolve) => {
    entered = resolve;
  });
  let release;
  const pending = new Promise((resolve) => {
    release = resolve;
  });
  await page.route("**/api/files/*?offset=0", async (route) => {
    const response = await route.fetch();
    entered();
    await pending;
    await route.fulfill({ response });
  });
  try {
    await page.locator("#files").setInputFiles({
      name: "pending.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("hello"),
    });
    await page
      .getByRole("button", { name: "Upload files", exact: true })
      .click();
    await started;
    await page
      .getByRole("button", { name: "Edit settings", exact: true })
      .click();
    await page
      .getByRole("checkbox", { name: "Delete after download", exact: true })
      .check();
    await page
      .getByRole("button", { name: "Save changes", exact: true })
      .click();
    await expect(
      page.getByText("Delete after download", { exact: true }).first(),
    ).toBeVisible();
    release();
    await page
      .getByRole("button", { name: "Publish share", exact: true })
      .click();
    await expect(page.locator("#share-url")).toBeVisible();
    const id = new URL(
      await page.locator("#share-url").inputValue(),
    ).pathname.split("/")[2];
    expect(
      await page.evaluate(
        async (id) =>
          (await (await fetch(`/api/shares/${id}`)).json())
            .delete_after_download,
        id,
      ),
    ).toBe(true);
  } finally {
    release();
  }
});

test("password protection, interrupted transfer recovery, and competing recipients", async ({
  page,
  browser,
}) => {
  await login(page);
  const share = await seed(page, {
    title: "Private one-time file",
    password: "deletion-share-password",
    deleteAfterDownload: true,
  });
  const context = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  const otherContext = await browser.newContext({
    viewport: page.viewportSize(),
    colorScheme: test.info().project.use.colorScheme,
  });
  try {
    const recipient = await context.newPage();
    await recipient.goto(share.url);
    await expect(
      recipient.getByRole("heading", { name: "Password required" }),
    ).toBeVisible();
    await expect(
      recipient.getByText("Delete after download", { exact: true }),
    ).toHaveCount(0);
    await recipient
      .getByLabel("Share password")
      .fill("deletion-share-password");
    await recipient
      .getByRole("button", { name: "Open share", exact: true })
      .click();
    await expect(
      recipient.getByRole("button", { name: "Download file once" }),
    ).toBeVisible();
    await expect(
      recipient.getByText("Incorrect password. Ask the sender or try again.", {
        exact: true,
      }),
    ).toHaveCount(0);
    await recipient.evaluate(async (share) => {
      const key = crypto.randomUUID();
      const response = await fetch(`/api/public/${share.id}/transfers`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          resource: share.files[0].id,
          password: "deletion-share-password",
          key,
        }),
      });
      const saved = { ...(await response.json()), key };
      sessionStorage.setItem(
        `binbin:${share.id}:${share.files[0].id}:public`,
        JSON.stringify(saved),
      );
      const partial = await fetch(`/d/${saved.token}`, {
        headers: { range: "bytes=0-1" },
      });
      if (partial.status !== 206 || (await partial.text()) !== "he")
        throw new Error("Cannot stage an interrupted transfer");
    }, share);
    const other = await otherContext.newPage();
    await other.goto(share.url);
    await other.getByLabel("Share password").fill("deletion-share-password");
    await other
      .getByRole("button", { name: "Open share", exact: true })
      .click();
    await expect(
      other.getByRole("heading", { name: "Download already in progress" }),
    ).toBeVisible();
    await expect(other.locator('[data-action="public-download"]')).toHaveCount(
      0,
    );
    await recipient.reload();
    await recipient
      .getByLabel("Share password")
      .fill("deletion-share-password");
    await recipient
      .getByRole("button", { name: "Open share", exact: true })
      .click();
    await expect(
      recipient.getByRole("heading", { name: "Continue your download" }),
    ).toBeVisible();
    await expect(
      recipient.getByText("Incorrect password. Ask the sender or try again.", {
        exact: true,
      }),
    ).toHaveCount(0);
    await screenshot(recipient, "delete-recovery");
    const downloading = recipient.waitForEvent("download");
    await recipient
      .getByRole("button", { name: "Retry download", exact: true })
      .click();
    expect(await (await downloading).failure()).toBeNull();
    await expect(
      recipient.getByRole("heading", {
        name: "Deleted after download",
        exact: true,
      }),
    ).toBeVisible();
    await expect
      .poll(() =>
        recipient.evaluate(() =>
          Object.keys(sessionStorage).some((key) => key.startsWith("binbin:")),
        ),
      )
      .toBe(false);
  } finally {
    await context.close();
    await otherContext.close();
  }
});
