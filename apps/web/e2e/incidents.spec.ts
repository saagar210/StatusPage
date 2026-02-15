import { test, expect } from "./fixtures/auth";

// These tests require authentication and a running API server + database.
// Set TEST_SESSION_TOKEN environment variable to run these tests.

const skipIfNoAuth = !process.env.TEST_SESSION_TOKEN;

test.describe("Incidents Management", () => {
  test.skip(skipIfNoAuth, "Requires TEST_SESSION_TOKEN environment variable");

  test("can create a new incident", async ({ authenticatedPage }) => {
    await authenticatedPage.goto("/dashboard/demo/incidents/new");
    await authenticatedPage.getByLabel(/title/i).fill("E2E Test Incident");
    await authenticatedPage.getByLabel(/message/i).fill("Investigating the issue from E2E test");
    await authenticatedPage.getByRole("button", { name: /create/i }).click();
    await expect(authenticatedPage.getByText("E2E Test Incident")).toBeVisible();
  });

  test("can view incident list", async ({ authenticatedPage }) => {
    await authenticatedPage.goto("/dashboard/demo/incidents");
    await expect(authenticatedPage.getByText(/incidents/i)).toBeVisible();
  });
});
