import { expect, test } from "./fixtures/auth";

const skipIfNoAuth = !process.env.TEST_SESSION_TOKEN;

test.describe("Settings Management", () => {
  test.skip(skipIfNoAuth, "Requires TEST_SESSION_TOKEN environment variable");

  test("can manage notification preferences and webhooks", async ({
    authenticatedPage,
    request,
  }) => {
    const apiUrl = process.env.API_URL || "http://127.0.0.1:4400";
    await request.post(`${apiUrl}/api/public/demo/subscribe`, {
      data: { email: "pending-subscriber@example.com" },
    });

    await authenticatedPage.goto("/dashboard/demo/settings");

    await expect(
      authenticatedPage.getByRole("heading", { name: "Settings" }),
    ).toBeVisible();
    await expect(
      authenticatedPage.getByText("Subscribers", { exact: true }).first(),
    ).toBeVisible();
    await expect(
      authenticatedPage.getByText("Recent Delivery Activity", { exact: true }),
    ).toBeVisible();
    await expect(
      authenticatedPage.getByText("pending-subscriber@example.com", {
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      authenticatedPage.getByRole("button", { name: "Resend Verification" }),
    ).toBeVisible();

    const serviceStatusWebhookToggle = authenticatedPage.getByLabel(
      "Webhook when service status changes",
    );
    await serviceStatusWebhookToggle.uncheck();
    await authenticatedPage
      .getByRole("button", { name: "Save Notification Preferences" })
      .click();
    await expect(
      authenticatedPage.getByText("Notification preferences saved"),
    ).toBeVisible();

    await authenticatedPage.getByLabel("Name").last().fill("Ops Bridge");
    await authenticatedPage
      .getByLabel("URL")
      .last()
      .fill("https://example.com/hooks/statuspage");
    await authenticatedPage.getByLabel("Signing secret").fill("supersecret123");
    await authenticatedPage.getByRole("button", { name: "Create Webhook" }).click();

    await expect(authenticatedPage.getByText("Ops Bridge")).toBeVisible();
    await expect(
      authenticatedPage.getByText("https://example.com/hooks/statuspage"),
    ).toBeVisible();

    await expect(
      authenticatedPage.getByLabel("Filter email deliveries"),
    ).toBeVisible();
    await expect(
      authenticatedPage.getByLabel("Filter webhook deliveries"),
    ).toBeVisible();

    await authenticatedPage
      .getByRole("button", { name: "Resend Verification" })
      .click();
    await expect(
      authenticatedPage.getByText("Queued another verification email"),
    ).toBeVisible();

    await authenticatedPage.getByRole("button", { name: "Disable" }).click();
    await expect(
      authenticatedPage.getByText("Disabled", { exact: true }).first(),
    ).toBeVisible();
  });
});
