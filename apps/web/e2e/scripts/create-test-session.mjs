import crypto from "node:crypto";
import process from "node:process";
import pg from "pg";

const databaseUrl = process.env.DATABASE_URL;

if (!databaseUrl) {
  console.error("DATABASE_URL must be set");
  process.exit(2);
}

const userEmail = process.env.E2E_USER_EMAIL || "demo@statuspage.sh";
const sessionDays = Number(process.env.E2E_SESSION_DAYS || "7");
const pool = new pg.Pool({ connectionString: databaseUrl });

try {
  const userResult = await pool.query(
    'SELECT id FROM users WHERE email = $1 LIMIT 1',
    [userEmail]
  );

  const userId = userResult.rows[0]?.id;

  if (!userId) {
    console.error(`No user found for ${userEmail}. Run the seed command first.`);
    process.exit(1);
  }

  const sessionToken = crypto.randomBytes(24).toString("hex");
  const expires = new Date(Date.now() + sessionDays * 24 * 60 * 60 * 1000);

  await pool.query('DELETE FROM sessions WHERE "userId" = $1', [userId]);
  await pool.query(
    'INSERT INTO sessions (id, "sessionToken", "userId", expires) VALUES (gen_random_uuid(), $1, $2, $3)',
    [sessionToken, userId, expires]
  );

  process.stdout.write(sessionToken);
} finally {
  await pool.end();
}
