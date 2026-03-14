import { expect } from "@playwright/test";

import { test } from "./fixtures/auth";

const skipIfNoAuth = !process.env.TEST_SESSION_TOKEN;

test.describe("Managed product flows", () => {
  test.skip(skipIfNoAuth, "Requires TEST_SESSION_TOKEN environment variable");

  test("can create a managed workspace through onboarding", async ({ authenticatedPage }) => {
    const suffix = Date.now().toString().slice(-6);
    const name = `Managed ${suffix}`;
    const slug = `managed-${suffix}`;

    await authenticatedPage.goto("/dashboard/onboarding");
    await expect(
      authenticatedPage.getByRole("heading", { name: "Create your managed workspace" }),
    ).toBeVisible();

    await authenticatedPage.getByLabel("Organization name").fill(name);
    await authenticatedPage.getByLabel("Status page slug").fill(slug);
    await authenticatedPage.getByRole("button", { name: "Continue" }).click();

    await authenticatedPage.getByLabel("First service").fill("API");
    await authenticatedPage.getByRole("button", { name: "Continue" }).click();

    await authenticatedPage.getByLabel("Monitor URL").fill("https://example.com/health");
    await authenticatedPage.getByRole("button", { name: "Continue" }).click();

    await authenticatedPage.getByLabel("Teammate email").fill("teammate@example.com");
    await authenticatedPage.getByRole("button", { name: "Continue" }).click();
    await authenticatedPage.getByRole("button", { name: "Continue" }).click();
    await authenticatedPage.getByRole("button", { name: "Create Managed Workspace" }).click();

    await expect(authenticatedPage).toHaveURL(new RegExp(`/dashboard/${slug}$`));
    await expect(authenticatedPage.getByText("Managed Setup Checklist")).toBeVisible();
  });

  test("can load the internal support console for the seeded demo org", async ({
    authenticatedPage,
  }) => {
    await authenticatedPage.goto("/dashboard/internal-support");
    await expect(
      authenticatedPage.getByRole("heading", { name: "Internal Support" }),
    ).toBeVisible();

    await authenticatedPage.getByLabel("Internal admin token").fill(
      process.env.INTERNAL_ADMIN_TOKEN || "managed-admin-token",
    );
    await authenticatedPage.getByLabel("Organization slug").fill("demo");
    await authenticatedPage.getByRole("button", { name: "Load support view" }).click();

    await expect(authenticatedPage.getByText("Organization snapshot")).toBeVisible();
    await expect(authenticatedPage.getByRole("button", { name: "Sync billing" })).toBeVisible();
  });
});
