import { test, expect } from "@playwright/test";
import { login } from "./support/session.js";

const png = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aZ1cAAAAASUVORK5CYII=",
  "base64",
);
const image = (name = "screenshot.png") => ({
  name,
  mimeType: "image/png",
  buffer: png,
});
const text = (name = "notes.txt", contents = "hello") => ({
  name,
  mimeType: "text/plain",
  buffer: Buffer.from(contents),
});
const rows = (page) => page.locator(".queued-file");
async function openNew(page) {
  await login(page);
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
}
async function start(page, files) {
  await openNew(page);
  await page.locator("#files").setInputFiles(files);
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
}

test("selection appends files, resolves collisions, previews images, and undoes removal", async ({
  page,
}) => {
  await openNew(page);
  await page.locator("#files").setInputFiles(image());
  await page.locator("#files").setInputFiles(image());
  await expect(rows(page)).toHaveCount(2);
  await expect(rows(page).nth(1)).toContainText("screenshot (2).png");
  await page.locator("#drop-zone").evaluate((element) => {
    const transfer = new DataTransfer();
    transfer.items.add(
      new File(["notes"], "notes.txt", { type: "text/plain" }),
    );
    element.dispatchEvent(
      new DragEvent("drop", {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer,
      }),
    );
  });
  await expect(rows(page)).toHaveCount(3);
  await page
    .getByRole("button", { name: "Preview screenshot.png", exact: true })
    .click();
  const viewer = page.getByRole("dialog");
  await expect(
    viewer.getByRole("button", { name: "Previous image", exact: true }),
  ).toBeDisabled();
  await expect(
    viewer.getByRole("button", { name: "Next image", exact: true }),
  ).toBeEnabled();
  await expect(
    viewer.getByRole("img", { name: "screenshot.png", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(
    viewer.getByRole("heading", { name: "screenshot (2).png", exact: true }),
  ).toBeVisible();
  const nextImage = viewer.getByRole("img", {
    name: "screenshot (2).png",
    exact: true,
  });
  await expect(nextImage).toBeVisible();
  await expect(
    viewer.getByRole("button", { name: "Previous image", exact: true }),
  ).toBeEnabled();
  await expect(
    viewer.getByRole("button", { name: "Next image", exact: true }),
  ).toBeDisabled();
  await expect
    .poll(() =>
      nextImage.evaluate((image) => image.complete && image.naturalWidth > 0),
    )
    .toBe(true);
  await page.screenshot({
    path: test.info().outputPath("image-viewer.png"),
    fullPage: true,
  });
  await page.keyboard.press("Escape");
  await expect(viewer).not.toBeVisible();
  await page
    .getByRole("button", { name: "Remove screenshot.png", exact: true })
    .click();
  await expect(rows(page)).toHaveCount(2);
  await page
    .getByRole("button", {
      name: "Undo removal of screenshot.png",
      exact: true,
    })
    .click();
  await expect(rows(page)).toHaveCount(3);
  await page.screenshot({
    path: test.info().outputPath("selected-images.png"),
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "Clear all files", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Keep files", exact: true })
    .click();
  await expect(rows(page)).toHaveCount(3);
  await page
    .getByRole("button", { name: "Clear all files", exact: true })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Clear all", exact: true })
    .click();
  await expect(rows(page)).toHaveCount(0);
});

test("draft files remain editable, additions wait for upload, and publication commits removals", async ({
  page,
}) => {
  await start(page, [image(), text()]);
  await page
    .getByRole("button", { name: "Remove notes.txt", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Undo removal of notes.txt", exact: true })
    .click();
  await expect(rows(page).filter({ hasText: "notes.txt" })).toHaveAttribute(
    "data-state",
    "uploaded",
  );
  await page
    .getByRole("button", { name: "Remove notes.txt", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Undo removal of notes.txt",
      exact: true,
    }),
  ).toBeEnabled();
  let chunks = 0;
  page.on("request", (request) => {
    if (request.method() === "PATCH" && request.url().includes("?offset="))
      chunks++;
  });
  await page.locator("#files").setInputFiles(text("extra.txt"));
  await expect(
    page.getByRole("button", { name: "Upload new files", exact: true }),
  ).toBeEnabled();
  expect(chunks).toBe(0);
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Upload new files", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Publish share", exact: true })
    .click();
  await expect(page.locator("#share-url")).toBeVisible();
  const published = await page.evaluate(async () =>
    (await fetch(`/api/shares/${location.hash.split("/")[1]}`)).json(),
  );
  expect(published.files.map((file) => file.name)).toEqual([
    "screenshot.png",
    "extra.txt",
  ]);
  expect(published.removed_files).toEqual([]);
});

test("files added during an upload join its queue while navigation remains available", async ({
  page,
}) => {
  await openNew(page);
  await page.locator("#files").setInputFiles(text("first.txt"));
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  let entered;
  const started = new Promise((resolve) => {
    entered = resolve;
  });
  let first = true;
  await page.route("**/api/files/*?offset=*", async (route) => {
    if (first) {
      first = false;
      entered();
      await gate;
    }
    await route.continue();
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await started;
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await page.locator("#active-uploads a").click();
  await page.locator("#files").setInputFiles(text("second.txt"));
  await expect(rows(page)).toHaveCount(2);
  release();
  await expect(rows(page).filter({ hasText: "second.txt" })).toHaveAttribute(
    "data-state",
    "uploaded",
  );
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
});

test("failed files leave other uploads running and retry independently", async ({
  page,
}) => {
  await openNew(page);
  await page
    .locator("#files")
    .setInputFiles([text("first.txt"), text("second.txt")]);
  let failed = false;
  await page.route("**/api/files/*?offset=*", async (route) => {
    if (!failed) {
      failed = true;
      return route.fulfill({
        status: 503,
        json: { error: "Temporary upload failure" },
      });
    }
    await route.continue();
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await expect(rows(page).filter({ hasText: "first.txt" })).toHaveAttribute(
    "data-state",
    "failed",
  );
  await expect(rows(page).filter({ hasText: "second.txt" })).toHaveAttribute(
    "data-state",
    "uploaded",
  );
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Retry first.txt", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
});

test("reload retains completed previews and resumes only the unfinished original", async ({
  page,
}) => {
  await start(page, [image(), text()]);
  const id = await page.evaluate(() => location.hash.split("/")[1]);
  const partial = await page.evaluate(async (id) => {
    const me = await (await fetch("/api/me")).json();
    const draft = await (
      await fetch(`/api/shares/${id}/files`, {
        method: "PATCH",
        headers: {
          "content-type": "application/json",
          "x-binbin-csrf": me.csrf,
        },
        body: JSON.stringify({ add: [{ name: "resume.txt", size: 5 }] }),
      })
    ).json();
    const file = draft.files.find((file) => file.name === "resume.txt");
    await fetch(`/api/files/${file.id}?offset=0`, {
      method: "PATCH",
      headers: { "x-binbin-csrf": me.csrf },
      body: "he",
    });
    return file;
  }, id);
  await page.reload();
  await expect(
    rows(page).filter({ hasText: "screenshot.png" }),
  ).toHaveAttribute("data-state", "uploaded");
  const thumb = page
    .getByRole("button", { name: "Preview screenshot.png", exact: true })
    .locator("img");
  await expect(thumb).toBeVisible();
  expect(
    await thumb.evaluate((image) => image.complete && image.naturalWidth > 0),
  ).toBe(true);
  await page
    .getByLabel("Original file for resume.txt", { exact: true })
    .setInputFiles(text("resume.txt", "xxxxx"));
  await page
    .getByRole("button", { name: "Resume upload", exact: true })
    .click();
  await expect(rows(page).filter({ hasText: "resume.txt" })).toHaveAttribute(
    "data-state",
    "failed",
  );
  await page
    .getByLabel("Original file for resume.txt", { exact: true })
    .setInputFiles(text("resume.txt", "hello"));
  await page
    .getByRole("button", { name: "Resume upload", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
  const draft = await page.evaluate(
    async (id) => (await fetch(`/api/shares/${id}`)).json(),
    id,
  );
  expect(draft.files.find((file) => file.id === partial.id).offset).toBe(5);
  expect(draft.files).toHaveLength(3);
});

test("screenshot paste appends images, respects fields, and preserves selection across navigation", async ({
  page,
}) => {
  await openNew(page);
  await page.locator("#files").setInputFiles(text());
  const paste = async (target) =>
    page.evaluate(
      ({ data, target }) => {
        const transfer = new DataTransfer();
        transfer.items.add(
          new File([Uint8Array.from(data)], "image.png", { type: "image/png" }),
        );
        document.querySelector(target).dispatchEvent(
          new ClipboardEvent("paste", {
            bubbles: true,
            cancelable: true,
            clipboardData: transfer,
          }),
        );
      },
      { data: [...png], target },
    );
  await paste('input[name="title"]');
  await expect(rows(page)).toHaveCount(1);
  await paste("#app");
  await expect(rows(page)).toHaveCount(2);
  await expect(rows(page).last()).toContainText(/Screenshot \d{4}-\d{2}-\d{2}/);
  await page.getByLabel("Title", { exact: true }).fill("Retained details");
  await page.getByLabel("Download allowance", { exact: true }).fill("3");
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await expect(rows(page)).toHaveCount(2);
  await expect(page.getByLabel("Title", { exact: true })).toHaveValue(
    "Retained details",
  );
  await expect(
    page.getByLabel("Download allowance", { exact: true }),
  ).toHaveValue("3");
});

test("removing an in-flight file lets the queue continue and Undo restores its progress", async ({
  page,
}) => {
  await openNew(page);
  await page
    .locator("#files")
    .setInputFiles([text("first.txt"), text("second.txt")]);
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  let entered;
  const started = new Promise((resolve) => {
    entered = resolve;
  });
  let first = true;
  await page.route("**/api/files/*?offset=*", async (route) => {
    if (first) {
      first = false;
      entered();
      await held;
    }
    await route.continue().catch(() => {});
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await started;
  await page
    .getByRole("button", { name: "Remove first.txt", exact: true })
    .click();
  await expect(rows(page)).toHaveCount(1);
  await expect(rows(page)).toHaveAttribute("data-state", "uploaded");
  release();
  await page
    .getByRole("button", { name: "Undo removal of first.txt", exact: true })
    .click();
  await expect(rows(page)).toHaveCount(2);
  await page
    .getByRole("button", { name: "Upload new files", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
});

test("pause preserves local files and resumes a chunk whose response was lost", async ({
  page,
}) => {
  await openNew(page);
  await page.locator("#files").setInputFiles(text());
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  let entered;
  const committed = new Promise((resolve) => {
    entered = resolve;
  });
  let first = true;
  let chunks = 0;
  await page.route("**/api/files/*?offset=*", async (route) => {
    chunks++;
    if (!first) return route.continue();
    first = false;
    const response = await route.fetch();
    entered();
    await held;
    await route.fulfill({ response }).catch(() => {});
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await committed;
  await page.getByRole("button", { name: /^Pause upload/ }).click();
  await expect(page.locator("#active-uploads")).not.toBeVisible();
  release();
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await page.getByRole("link", { name: "Shares", exact: true }).click();
  await page
    .locator(".share-row")
    .filter({ hasText: "notes.txt" })
    .first()
    .click();
  await expect(rows(page)).toHaveAttribute("data-state", "uploaded");
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
  expect(chunks).toBe(1);
});

test("file limits are visible before uploading and invalid files can be removed", async ({
  page,
}) => {
  await login(page);
  await page.route("**/api/settings", async (route) => {
    const response = await route.fetch();
    const data = await response.json();
    data.settings.file_limit = 4;
    await route.fulfill({ response, json: data });
  });
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await page.locator("#files").setInputFiles(text("too-large.txt"));
  await expect(rows(page)).toContainText("Exceeds the 4 B file limit");
  await expect(
    page.getByRole("button", { name: "Upload files", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Remove too-large.txt", exact: true })
    .click();
  await page.locator("#files").setInputFiles(text("fits.txt", "yes"));
  await expect(
    page.getByRole("button", { name: "Upload files", exact: true }),
  ).toBeEnabled();
});

test("lost draft edit responses reconcile IDs and removals before retry", async ({
  page,
}) => {
  await start(page, text());
  let lose = true;
  await page.route("**/api/shares/*/files", async (route) => {
    const response = await route.fetch();
    if (lose) {
      lose = false;
      return route.fulfill({
        status: 503,
        json: { error: "Draft response lost" },
      });
    }
    return route.fulfill({ response });
  });
  await page.locator("#files").setInputFiles(text("extra.txt"));
  await page
    .getByRole("button", { name: "Upload new files", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
  await expect(rows(page)).toHaveCount(2);
  lose = true;
  await page
    .getByRole("button", { name: "Remove notes.txt", exact: true })
    .click();
  const undo = page.getByRole("button", {
    name: "Undo removal of notes.txt",
    exact: true,
  });
  await expect(undo).toBeEnabled();
  await expect(rows(page)).toHaveCount(1);
  lose = true;
  await undo.click();
  await expect(rows(page)).toHaveCount(2);
  await expect(rows(page).filter({ hasText: "notes.txt" })).toHaveAttribute(
    "data-state",
    "uploaded",
  );
});

test("fresh draft metadata reconciles external edits and ignores older delayed snapshots", async ({
  page,
}) => {
  await start(page, text());
  const id = await page.evaluate(() => location.hash.split("/")[1]);
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await page.evaluate(async (id) => {
    const me = await (await fetch("/api/me")).json();
    await fetch(`/api/shares/${id}/files`, {
      method: "PATCH",
      headers: { "content-type": "application/json", "x-binbin-csrf": me.csrf },
      body: JSON.stringify({ add: [{ name: "external.txt", size: 0 }] }),
    });
  }, id);
  await page.goto(`/#share/${id}`);
  await expect(rows(page)).toHaveCount(2);
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  let entered;
  const captured = new Promise((resolve) => {
    entered = resolve;
  });
  await page.route(`**/api/shares/${id}`, async (route) => {
    const response = await route.fetch();
    entered();
    await held;
    await route.fulfill({ response });
  });
  await page.goto(`/#share/${id}`);
  await captured;
  await page.locator("#files").setInputFiles(text("newer.txt", ""));
  await expect(rows(page)).toHaveCount(3);
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
  const received = page.waitForResponse(
    (response) =>
      response.request().method() === "GET" &&
      response.url().endsWith(`/api/shares/${id}`),
  );
  release();
  await (await received).finished();
  await page.evaluate(() => new Promise(requestAnimationFrame));
  await expect(rows(page)).toHaveCount(3);
  await expect(rows(page).filter({ hasText: "newer.txt" })).toBeVisible();
});

test("failed logout preserves a pasted selection and paste works on the file input", async ({
  page,
}) => {
  await openNew(page);
  await page.locator("#files").focus();
  await page.locator("#files").evaluate(
    (input, data) => {
      const transfer = new DataTransfer();
      transfer.items.add(
        new File([Uint8Array.from(data)], "image.png", { type: "image/png" }),
      );
      input.dispatchEvent(
        new ClipboardEvent("paste", {
          bubbles: true,
          cancelable: true,
          clipboardData: transfer,
        }),
      );
    },
    [...png],
  );
  await expect(rows(page)).toHaveCount(1);
  await page.route("**/api/logout", (route) =>
    route.fulfill({ status: 503, json: { error: "Temporary logout failure" } }),
  );
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.locator("#notice")).toContainText(
    "Temporary logout failure",
  );
  await expect(rows(page)).toHaveCount(1);
  await page.getByRole("button", { name: /^Preview Screenshot/ }).click();
  await expect(page.getByRole("dialog").getByRole("img")).toBeVisible();
});

test("Undo uses server-relative time when the browser clock is ahead", async ({
  page,
}) => {
  await start(page, [text(), text("extra.txt")]);
  await page.evaluate(() => {
    const now = Date.now;
    Date.now = () => now() + 600000;
  });
  await page
    .getByRole("button", { name: "Remove notes.txt", exact: true })
    .click();
  const undo = page.getByRole("button", {
    name: "Undo removal of notes.txt",
    exact: true,
  });
  await expect(undo).toBeEnabled();
  await undo.click();
  await expect(rows(page)).toHaveCount(2);
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
});

test("an interrupted verifier cannot overwrite a resumed upload", async ({
  page,
}) => {
  await openNew(page);
  const id = await page.evaluate(async () => {
    const me = await (await fetch("/api/me")).json();
    const draft = await (
      await fetch("/api/shares", {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-binbin-csrf": me.csrf,
        },
        body: JSON.stringify({
          title: "Verifier race",
          files: [{ name: "resume.txt", size: 5 }],
          days: 7,
          limit: null,
          password: null,
        }),
      })
    ).json();
    await fetch(`/api/files/${draft.files[0].id}?offset=0`, {
      method: "PATCH",
      headers: { "x-binbin-csrf": me.csrf },
      body: "he",
    });
    const original = crypto.subtle.digest.bind(crypto.subtle);
    let first = true;
    crypto.subtle.digest = async (...args) => {
      const result = await original(...args);
      if (first) {
        first = false;
        window.verifierReady = true;
        await new Promise((resolve) => {
          window.releaseVerifier = resolve;
        });
      }
      return result;
    };
    return draft.id;
  });
  await page.goto(`/#share/${id}`);
  await page.locator("#files").setInputFiles(text("resume.txt"));
  await page
    .getByRole("button", { name: "Resume upload", exact: true })
    .click();
  await expect.poll(() => page.evaluate(() => window.verifierReady)).toBe(true);
  await page.getByRole("button", { name: /^Pause upload/ }).click();
  await page
    .getByRole("button", { name: "Resume upload", exact: true })
    .click();
  await expect(rows(page)).toHaveAttribute("data-state", "uploaded");
  await page.evaluate(async () => {
    window.releaseVerifier();
    await new Promise(requestAnimationFrame);
  });
  await expect(rows(page)).toHaveAttribute("data-state", "uploaded");
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeEnabled();
});
