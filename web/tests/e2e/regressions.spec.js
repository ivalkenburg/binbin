import { test, expect } from "@playwright/test";
import { login, signIn } from "./support/session.js";
import { seed } from "./support/shares.js";

function gate() {
  let release;
  const promise = new Promise((resolve) => (release = resolve));
  return { promise, release };
}

async function chooseFile(page) {
  await page
    .getByRole("link", { name: "New share", exact: true })
    .first()
    .click();
  await page.locator("#files").setInputFiles({
    name: "race.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("hello"),
  });
}

test("draft creation survives navigation before its first response", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await login(page);
  await chooseFile(page);
  const entered = gate();
  const waiting = gate();
  await page.route("**/api/shares", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    const response = await route.fetch();
    entered.release();
    await waiting.promise;
    await route.fulfill({ response });
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await entered.promise;
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(page).toHaveURL(/#settings$/);
  await expect(page.locator("#active-uploads")).toContainText(
    "Creating private draft",
  );
  const completed = page.waitForResponse(
    (response) =>
      response.request().method() === "PATCH" &&
      response.url().includes("?offset="),
  );
  waiting.release();
  await completed;
  await expect(page).toHaveURL(/#settings$/);
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("signing out cancels pending draft setup without locking later uploads", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await login(page);
  await chooseFile(page);
  const entered = gate();
  const waiting = gate();
  await page.route("**/api/shares", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    const response = await route.fetch();
    entered.release();
    await waiting.promise;
    await route.fulfill({ response }).catch(() => {});
  });
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await entered.promise;
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.locator("#login")).toBeVisible();
  waiting.release();
  await page.unroute("**/api/shares");
  await signIn(page);
  await chooseFile(page);
  await page.getByRole("button", { name: "Upload files", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Publish share", exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("late share responses cannot replace Settings and skip links preserve the route", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page, { title: "Navigation regression" });
  const entered = gate();
  const waiting = gate();
  const finished = gate();
  await page.route(`**/api/shares/${share.id}`, async (route) => {
    const response = await route.fetch();
    entered.release();
    await waiting.promise;
    await route.fulfill({ response });
    finished.release();
  });
  await page.evaluate((id) => (location.hash = `share/${id}`), share.id);
  await entered.promise;
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  waiting.release();
  await finished.promise;
  await page.unroute(`**/api/shares/${share.id}`);
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await page.goto(`/#share/${share.id}`);
  await expect(
    page.getByRole("heading", { name: share.title, exact: true }),
  ).toBeVisible();
  await page.locator(".skip-link").focus();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(new RegExp(`#share/${share.id}$`));
  await expect(page.locator("#app")).toBeFocused();
});

test("double clicks share issuance and unavailable storage still permits retries", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await page.goto(share.url);
  await page.evaluate(() => {
    Storage.prototype.getItem = () => {
      throw new Error("Storage denied");
    };
    Storage.prototype.setItem = () => {
      throw new Error("Storage denied");
    };
  });
  const entered = gate();
  const waiting = gate();
  let issued = 0;
  let bodies = 0;
  page.on("download", () => bodies++);
  await page.route("**/transfers", async (route) => {
    issued++;
    const response = await route.fetch();
    entered.release();
    await waiting.promise;
    await route.fulfill({ response });
  });
  const button = page.locator('[data-action="public-download"]').first();
  await button.click();
  await entered.promise;
  await button.click();
  const downloading = page.waitForEvent("download");
  waiting.release();
  await downloading;
  await expect(
    page.getByRole("link", { name: "Retry this download" }),
  ).toBeVisible();
  expect(issued).toBe(1);
  expect(bodies).toBe(1);
  const retrying = page.waitForEvent("download");
  await button.click();
  await retrying;
  expect(issued).toBe(1);
});

test("password changes replace rejected cached capabilities", async ({
  page,
  context,
}) => {
  await login(page);
  const share = await seed(page, { password: "initial-password" });
  const recipient = await context.newPage();
  await recipient.goto(share.url);
  await recipient
    .getByLabel("Share password", { exact: true })
    .fill("initial-password");
  await recipient.getByRole("button", { name: "Open share" }).click();
  let downloading = recipient.waitForEvent("download");
  await recipient.locator('[data-action="public-download"]').first().click();
  await downloading;
  const original = await recipient
    .getByRole("link", { name: "Retry this download" })
    .getAttribute("href");
  await page.evaluate(async (share) => {
    const me = await (await fetch("/api/me")).json();
    const response = await fetch(`/api/shares/${share.id}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", "x-binbin-csrf": me.csrf },
      body: JSON.stringify({
        title: share.title,
        expires: share.expires,
        limit: share.limit,
        password: "replacement-password",
        change_password: true,
      }),
    });
    if (!response.ok) throw new Error(await response.text());
  }, share);
  await recipient.reload();
  await recipient
    .getByLabel("Share password", { exact: true })
    .fill("replacement-password");
  await recipient.getByRole("button", { name: "Open share" }).click();
  downloading = recipient.waitForEvent("download");
  await recipient.locator('[data-action="public-download"]').first().click();
  await downloading;
  await expect(
    recipient.getByRole("link", { name: "Retry this download" }),
  ).not.toHaveAttribute("href", original);
});

test("management downloads get a new capability after logout and login", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await page.goto(`/#share/${share.id}`);
  const tokens = [];
  page.on("response", async (response) => {
    if (response.url().endsWith("/transfers") && response.ok())
      tokens.push((await response.json()).token);
  });
  let downloading = page.waitForEvent("download");
  await page.locator('[data-action="download"]').first().click();
  await downloading;
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.locator("#login")).toBeVisible();
  await login(page);
  await page.goto(`/#share/${share.id}`);
  downloading = page.waitForEvent("download");
  await page.locator('[data-action="download"]').first().click();
  await downloading;
  await expect.poll(() => tokens.length).toBe(2);
  expect(tokens[0]).not.toBe(tokens[1]);
});

test("logout aborts stalled resume verification before a new sign-in", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await login(page);
  const draft = await seed(page, { partial: true });
  await page.goto(`/#share/${draft.id}`);
  await page.locator("#files").setInputFiles({
    name: "handover.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("hello"),
  });
  const entered = gate();
  const waiting = gate();
  await page.route("**/proofs", async (route) => {
    entered.release();
    await waiting.promise;
    await route.continue().catch(() => {});
  });
  await page
    .getByRole("button", { name: "Resume upload", exact: true })
    .click();
  await entered.promise;
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.locator("#login")).toBeVisible();
  await signIn(page);
  await expect(
    page.getByRole("heading", { name: "Resume this upload" }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  waiting.release();
  expect(errors).toEqual([]);
});

test("Base UI checkboxes submit account permissions and password changes", async ({
  page,
}) => {
  await login(page);
  await page.getByRole("link", { name: "Administration", exact: true }).click();
  const username = "base_ui_admin";
  await page.getByLabel("Username", { exact: true }).fill(username);
  await page
    .getByLabel("Initial password", { exact: true })
    .fill("base-ui-admin-password");
  await page.getByRole("checkbox", { name: "Administrator access" }).check();
  await page
    .getByRole("button", { name: "Create account", exact: true })
    .click();
  const row = page.locator(".user-row").filter({ hasText: username });
  await expect(row).toContainText("Administrator");
  const share = await seed(page, { title: "Checkbox password replacement" });
  await page.goto(`/#share/${share.id}`);
  await page
    .getByRole("button", { name: "Edit settings", exact: true })
    .click();
  await page
    .getByRole("checkbox", { name: "Replace or remove password" })
    .check();
  await page
    .getByLabel("New password", { exact: true })
    .fill("base-ui-share-password");
  await page.getByRole("button", { name: "Save changes", exact: true }).click();
  await expect(page.locator("#share-overview")).toContainText("Required");
});

async function adminRequest(page, path, method, data) {
  const session = await (await page.request.get("/api/me")).json();
  const response = await page.request.fetch(path, {
    method,
    headers: { "x-binbin-csrf": session.csrf },
    data,
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return response.json();
}

test("invalidated sessions leave management on mutations and sign out", async ({
  page,
  browser,
}) => {
  await login(page);
  const account = await adminRequest(page, "/api/users", "POST", {
    username: "session_invalidation",
    password: "initial-session-password",
    admin: false,
  });
  const context = await browser.newContext({
    baseURL: "http://localhost:8086",
  });
  const uploader = await context.newPage();
  try {
    await uploader.goto("/");
    await uploader
      .getByLabel("Username", { exact: true })
      .fill(account.username);
    await uploader
      .getByLabel("Password", { exact: true })
      .fill("initial-session-password");
    await uploader
      .getByRole("button", { name: "Sign in", exact: true })
      .click();
    await uploader.getByRole("link", { name: "Settings", exact: true }).click();
    await expect(uploader.getByLabel("Token name")).toBeVisible();
    await adminRequest(page, `/api/users/${account.id}`, "PATCH", {
      password: "replacement-session-password",
      enabled: true,
    });
    await uploader.getByLabel("Token name").fill("invalidated credential");
    await uploader
      .getByRole("button", { name: "Create token", exact: true })
      .click();
    await expect(uploader.locator("#login")).toBeVisible();
    await uploader
      .getByLabel("Username", { exact: true })
      .fill(account.username);
    await uploader
      .getByLabel("Password", { exact: true })
      .fill("replacement-session-password");
    await uploader
      .getByRole("button", { name: "Sign in", exact: true })
      .click();
    await expect(uploader.getByLabel("Token name")).toBeVisible();
    await adminRequest(page, `/api/users/${account.id}`, "PATCH", {
      password: "second-replacement-password",
      enabled: true,
    });
    await uploader
      .getByRole("button", { name: "Sign out", exact: true })
      .click();
    await expect(uploader.locator("#login")).toBeVisible();
  } finally {
    await context.close();
  }
});

test("switching reset targets clears the password and identifies the account", async ({
  page,
}) => {
  await login(page);
  const accounts = [];
  for (const username of ["reset_target_alice", "reset_target_bob"])
    accounts.push(
      await adminRequest(page, "/api/users", "POST", {
        username,
        password: "original-target-password",
        admin: false,
      }),
    );
  await page.getByRole("link", { name: "Administration", exact: true }).click();
  const first = page
    .locator(".user-row")
    .filter({ hasText: accounts[0].username });
  const second = page
    .locator(".user-row")
    .filter({ hasText: accounts[1].username });
  await first
    .getByRole("button", { name: "Reset password", exact: true })
    .click();
  await expect(
    page.getByRole("heading", {
      name: `Set a new password for ${accounts[0].username}`,
    }),
  ).toBeVisible();
  await page
    .locator("#password-reset")
    .getByLabel("New password")
    .fill("password-intended-for-alice");
  await second
    .getByRole("button", { name: "Reset password", exact: true })
    .click();
  await expect(
    page.getByRole("heading", {
      name: `Set a new password for ${accounts[1].username}`,
    }),
  ).toBeVisible();
  await expect(
    page.locator("#password-reset").getByLabel("New password"),
  ).toHaveValue("");
  const entered = gate(),
    waiting = gate();
  await page.route(`**/api/users/${accounts[1].id}`, async (route) => {
    entered.release();
    await waiting.promise;
    await route.continue();
  });
  await page
    .locator("#password-reset")
    .getByLabel("New password")
    .fill("password-intended-for-bob");
  await page
    .getByRole("button", { name: "Reset password and invalidate sessions" })
    .click();
  await entered.promise;
  await expect(
    first.getByRole("button", { name: "Reset password", exact: true }),
  ).toBeDisabled();
  waiting.release();
  await expect(page.locator("#password-reset")).not.toBeVisible();
});

test("valid small byte quotas do not block unrelated settings changes", async ({
  page,
}) => {
  await login(page);
  const original = (await (await page.request.get("/api/settings")).json())
    .settings;
  try {
    await adminRequest(page, "/api/settings", "PUT", {
      ...original,
      file_limit: 2 ** 20,
    });
    await page
      .getByRole("link", { name: "Administration", exact: true })
      .click();
    await expect(page.getByLabel("Maximum file size (GiB)")).toHaveValue(
      "0.0009765625",
    );
    await page.getByLabel("Default lifetime (days)").fill("8");
    expect(
      await page
        .locator("#instance-settings")
        .evaluate((form) => form.checkValidity()),
    ).toBeTruthy();
    await page
      .getByRole("button", { name: "Save limits", exact: true })
      .click();
    await expect(page.locator("#notice")).toContainText(
      "Instance limits saved.",
    );
    const updated = (await (await page.request.get("/api/settings")).json())
      .settings;
    expect(updated.file_limit).toBe(2 ** 20);
    expect(updated.default_days).toBe(8);
  } finally {
    await adminRequest(page, "/api/settings", "PUT", original);
  }
});

test("a failed refresh retains data with a visible retry and then recovers", async ({
  page,
}) => {
  await login(page);
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await page.getByLabel("Token name").fill("refresh failure regression");
  await page.getByRole("button", { name: "Create token", exact: true }).click();
  const row = page
    .locator(".token-row")
    .filter({ hasText: "refresh failure regression" });
  await expect(row).toBeVisible();
  await page.route(
    "**/api/tokens",
    (route) =>
      route.request().method() === "GET"
        ? route.fulfill({
            status: 503,
            contentType: "application/json",
            body: JSON.stringify({ error: "Temporary refresh failure" }),
          })
        : route.continue(),
    { times: 1 },
  );
  await row.getByRole("button", { name: "Revoke", exact: true }).click();
  await expect(
    page.getByText("Couldn’t refresh. Details shown may be out of date."),
  ).toBeVisible();
  await expect(row).toBeVisible();
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(row).not.toBeVisible();
  await expect(
    page.getByText("Couldn’t refresh. Details shown may be out of date."),
  ).not.toBeVisible();
});

test("cached server-valid retries survive local issuance expiry with one HEAD", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  let issued = 0,
    heads = 0;
  page.on("request", (request) => {
    if (request.url().endsWith("/transfers")) issued++;
    if (request.method() === "HEAD" && request.url().includes("/d/")) heads++;
  });
  await page.goto(share.url);
  let completed = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  await completed;
  const href = await page
    .getByRole("link", { name: "Retry this download" })
    .getAttribute("href");
  await page.evaluate(() => {
    const now = Date.now();
    Date.now = () => now + 25 * 60 * 60 * 1000;
  });
  completed = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  await completed;
  expect(issued).toBe(1);
  expect(heads).toBe(2);
  await expect(
    page.getByRole("link", { name: "Retry this download" }),
  ).toHaveAttribute("href", href);
  const current = await (
    await page.request.get(`/api/shares/${share.id}`)
  ).json();
  expect(current.used).toBe(1);
});

test("an expired pending issuance key can be replaced without reviving its URL", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await page.goto(share.url);
  const resource = share.files[0].id;
  await page.evaluate(
    ({ shareId, resource }) => {
      sessionStorage.setItem(
        `binbin:${shareId}:${resource}:public`,
        JSON.stringify({ key: "expired-key-with-lost-issuance-response" }),
      );
    },
    { shareId: share.id, resource },
  );
  const keys = [];
  await page.route("**/transfers", (route) => {
    keys.push(route.request().postDataJSON().key);
    return keys.length === 1
      ? route.fulfill({
          status: 404,
          contentType: "application/json",
          body: JSON.stringify({ error: "Share unavailable" }),
        })
      : route.continue();
  });
  const completed = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  await completed;
  expect(keys).toHaveLength(2);
  expect(keys[0]).toBe("expired-key-with-lost-issuance-response");
  expect(keys[1]).not.toBe(keys[0]);
  const current = await (
    await page.request.get(`/api/shares/${share.id}`)
  ).json();
  expect(current.used).toBe(1);
});

test("an exhausted public share can recover its charged retry after reload", async ({
  page,
}) => {
  await login(page);
  const share = await seed(page);
  await adminRequest(page, `/api/shares/${share.id}`, "PATCH", {
    title: share.title,
    expires: share.expires,
    limit: 1,
    password: null,
    change_password: false,
  });
  let issued = 0;
  page.on("request", (request) => {
    if (request.url().endsWith("/transfers")) issued++;
  });
  await page.goto(share.url);
  let completed = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  await completed;
  const href = await page
    .getByRole("link", { name: "Retry this download" })
    .getAttribute("href");
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Resume your download" }),
  ).toBeVisible();
  const retry = page.getByRole("link", { name: "Retry download", exact: true });
  await expect(retry).toHaveAttribute("href", href);
  completed = page.waitForEvent("download");
  await retry.click();
  await completed;
  expect(issued).toBe(1);
  const current = await (
    await page.request.get(`/api/shares/${share.id}`)
  ).json();
  expect(current.used).toBe(1);
});

test("reloaded public shares do not recover expired or revoked capabilities", async ({
  page,
}) => {
  await login(page);
  for (const reason of ["expiry", "revocation"]) {
    const share = await seed(page);
    await page.goto(share.url);
    const completed = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    await completed;
    if (reason === "revocation")
      await adminRequest(page, `/api/shares/${share.id}`, "DELETE");
    else
      await adminRequest(page, `/api/shares/${share.id}`, "PATCH", {
        title: share.title,
        expires: 1,
        limit: share.limit,
        password: null,
        change_password: false,
      });
    await page.reload();
    await expect(
      page.getByRole("heading", { name: "Share unavailable" }),
    ).toBeVisible();
    await expect(
      page.getByRole("status", { name: "Checking previous downloads…" }),
    ).not.toBeVisible();
    await expect(
      page.getByText("Checking previous downloads…"),
    ).not.toBeVisible();
    await expect(
      page.getByRole("link", { name: "Retry download", exact: true }),
    ).toHaveCount(0);
  }
});
