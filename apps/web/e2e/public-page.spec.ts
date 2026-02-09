import { test, expect } from "@playwright/test";

test.describe("Public Status Page", () => {
  test("shows 404 for non-existent org", async ({ page }) => {
    await page.goto("/s/nonexistent-org-xyz");
    await expect(page).toHaveTitle(/not found/i);
  });

  test("landing page renders with hero section", async ({ page }) => {
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: /status pages/i })
    ).toBeVisible();
    await expect(
      page.getByRole("link", { name: /get started/i })
    ).toBeVisible();
  });

  test("login page renders with GitHub button", async ({ page }) => {
    await page.goto("/login");
    await expect(
      page.getByRole("button", { name: /github/i })
    ).toBeVisible();
  });

  test("dashboard redirects to login when unauthenticated", async ({
    page,
  }) => {
    await page.goto("/dashboard");
    await page.waitForURL(/\/login/);
    expect(page.url()).toContain("/login");
  });
});
