import { expect } from "@playwright/test";
export async function login(page) {
  await page.goto("/");
  await signIn(page);
  await expect(
    page.getByRole("heading", { name: "Your shares" }),
  ).toBeVisible();
}

export async function signIn(page) {
  await page.getByLabel("Username", { exact: true }).fill("admin");
  await page
    .getByLabel("Password", { exact: true })
    .fill("browser-test-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
}
