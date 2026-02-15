import { test, expect } from "./fixtures/auth";

// These tests require authentication and a running API server + database.
// Set TEST_SESSION_TOKEN environment variable to run these tests.

const skipIfNoAuth = !process.env.TEST_SESSION_TOKEN;

test.describe("Services Management", () => {
  test.skip(skipIfNoAuth, "Requires TEST_SESSION_TOKEN environment variable");

  test("can create a new service", async ({ authenticatedPage }) => {
    await authenticatedPage.goto("/dashboard/demo/services");
    await authenticatedPage.getByRole("button", { name: /add service/i }).click();
    await authenticatedPage.getByLabel(/name/i).fill("Test Service E2E");
    await authenticatedPage.getByLabel(/description/i).fill("A test service from E2E");
    await authenticatedPage.getByRole("button", { name: /create/i }).click();
    await expect(authenticatedPage.getByText("Test Service E2E")).toBeVisible();
  });

  test("can view service list", async ({ authenticatedPage }) => {
    await authenticatedPage.goto("/dashboard/demo/services");
    // Should see services from seeded data
    await expect(authenticatedPage.getByText(/operational/i).first()).toBeVisible();
  });
});
