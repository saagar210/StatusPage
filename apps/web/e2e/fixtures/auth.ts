import { test as base } from "@playwright/test";

/**
 * Extended test fixtures with authentication support.
 *
 * Usage:
 *   import { test, expect } from './fixtures/auth';
 *
 *   test('authenticated test', async ({ authenticatedPage }) => {
 *     await authenticatedPage.goto('/dashboard');
 *   });
 */

type AuthFixtures = {
  authenticatedPage: any;
};

export const test = base.extend<AuthFixtures>({
  authenticatedPage: async ({ page }, use) => {
    // In a full implementation, we'd:
    // 1. Create a test user via API
    // 2. Get session token
    // 3. Set session cookie in browser
    //
    // For now, we assume the database is seeded with a "demo" org
    // and we'd manually set up auth session

    const sessionToken = process.env.TEST_SESSION_TOKEN;

    if (sessionToken) {
      // Set the session cookie
      await page.context().addCookies([
        {
          name: "authjs.session-token",
          value: sessionToken,
          domain: "localhost",
          path: "/",
          httpOnly: true,
          secure: false,
          sameSite: "Lax",
        },
      ]);
    } else {
      console.warn(
        "⚠️  No TEST_SESSION_TOKEN provided. Authenticated tests will fail."
      );
    }

    await use(page);
  },
});

export { expect } from "@playwright/test";
