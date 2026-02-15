import { chromium, FullConfig } from "@playwright/test";
import { execSync } from "child_process";

/**
 * Global setup runs once before all tests.
 * Sets up database and creates test data.
 */
async function globalSetup(config: FullConfig) {
  console.log("🔧 Running E2E global setup...");

  // Check if API is running
  const apiUrl = process.env.API_URL || "http://localhost:4000";
  console.log(`Checking API health at ${apiUrl}...`);

  try {
    const response = await fetch(`${apiUrl}/health`);
    if (!response.ok) {
      throw new Error(`API health check failed: ${response.status}`);
    }
    console.log("✓ API is healthy");
  } catch (error) {
    console.error("❌ API is not running. Please start it with: pnpm run dev:api");
    throw error;
  }

  // Run database migrations (idempotent)
  console.log("Running database migrations...");
  try {
    execSync("pnpm run db:migrate", {
      cwd: process.cwd() + "/../..",
      stdio: "inherit",
    });
    console.log("✓ Migrations complete");
  } catch (error) {
    console.error("❌ Migration failed:", error);
    throw error;
  }

  // Seed database with test data
  console.log("Seeding database with test data...");
  try {
    execSync("cargo run -p api-server --bin seed", {
      cwd: process.cwd() + "/../..",
      stdio: "inherit",
    });
    console.log("✓ Database seeded");
  } catch (error) {
    console.error("❌ Seeding failed:", error);
    throw error;
  }

  // Create authenticated session for testing
  console.log("Creating test session...");
  const browser = await chromium.launch();
  const context = await browser.newContext();
  const page = await context.newPage();

  // TODO: In a real setup, we'd create a test user session here
  // For now, we'll rely on seeded data and manual auth setup
  console.log("⚠️  Note: Authenticated tests require manual session setup");

  await browser.close();

  console.log("✅ Global setup complete!");
}

export default globalSetup;
