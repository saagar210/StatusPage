import { test, expect } from "@playwright/test";

// These tests require authentication and a running API server + database.
// They serve as a template for when the full stack is running.

test.describe("Services Management", () => {
  test.skip(true, "Requires authenticated session and running API server");

  test("can create a new service", async ({ page }) => {
    await page.goto("/dashboard/demo/services");
    await page.getByRole("button", { name: /add service/i }).click();
    await page.getByLabel(/name/i).fill("Test Service");
    await page.getByLabel(/description/i).fill("A test service");
    await page.getByRole("button", { name: /create/i }).click();
    await expect(page.getByText("Test Service")).toBeVisible();
  });

  test("can update a service status", async ({ page }) => {
    await page.goto("/dashboard/demo/services");
    // Would need existing service to test status change
    await expect(page.getByText(/operational/i).first()).toBeVisible();
  });
});
