import { execSync } from "child_process";

async function globalSetup() {
  console.log("🔧 Running E2E global setup...");

  const needsBackendBootstrap =
    process.env.E2E_WITH_BACKEND === "true" ||
    Boolean(process.env.TEST_SESSION_TOKEN);

  if (!needsBackendBootstrap) {
    console.log("ℹ️  Skipping backend bootstrap for public-only Playwright run");
    return;
  }

  if (process.env.TEST_SESSION_TOKEN) {
    console.log("ℹ️  Using pre-provisioned backend and deterministic test session");
    return;
  }

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

  console.log(
    "⚠️  This path only prepares backend data. Use `pnpm e2e:auth` for authenticated browser runs.",
  );

  console.log("✅ Global setup complete!");
}

export default globalSetup;
