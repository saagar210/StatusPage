use std::time::Duration;

use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use shared::enums::{IncidentImpact, IncidentStatus, MemberRole, ServiceStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await?;

    println!("Running migrations...");
    sqlx::migrate!("../../migrations").run(&pool).await?;

    println!("Seeding data...");

    // 1. Create test user
    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO users (id, name, email, "emailVerified", image)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (email) DO UPDATE SET name = $2
           RETURNING id"#,
    )
    .bind(user_id)
    .bind("Demo User")
    .bind("demo@statuspage.sh")
    .bind(Utc::now())
    .bind("https://avatars.githubusercontent.com/u/0?v=4")
    .execute(&pool)
    .await?;
    println!("  Created user: Demo User (demo@statuspage.sh)");

    // 2. Create organization
    let org_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, name, slug, brand_color)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (slug) DO UPDATE SET name = $2
         RETURNING id",
    )
    .bind(org_id)
    .bind("Demo Company")
    .bind("demo")
    .bind("#3B82F6")
    .execute(&pool)
    .await?;
    println!("  Created organization: Demo Company (slug: demo)");

    // 3. Add user as owner
    sqlx::query(
        "INSERT INTO members (org_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (org_id, user_id) DO NOTHING",
    )
    .bind(org_id)
    .bind(user_id)
    .bind(MemberRole::Owner.to_string())
    .execute(&pool)
    .await?;

    // 4. Create services
    let services = [
        ("API", "Core REST API endpoints", "Core Infrastructure", ServiceStatus::Operational),
        ("Web Application", "Customer-facing web app", "Core Infrastructure", ServiceStatus::Operational),
        ("Database", "PostgreSQL primary database", "Core Infrastructure", ServiceStatus::Operational),
        ("CDN", "Static asset delivery", "Edge Services", ServiceStatus::Operational),
        ("Email Service", "Transactional email delivery", "Communications", ServiceStatus::DegradedPerformance),
    ];

    let mut service_ids = Vec::new();
    for (i, (name, desc, group, status)) in services.iter().enumerate() {
        let sid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO services (id, org_id, name, description, group_name, current_status, display_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(sid)
        .bind(org_id)
        .bind(name)
        .bind(desc)
        .bind(group)
        .bind(status.to_string())
        .bind(i as i32)
        .execute(&pool)
        .await?;
        service_ids.push((sid, *name));
        println!("  Created service: {} ({})", name, status);
    }

    // 5. Create a resolved incident (happened 3 days ago, lasted 45 min)
    let resolved_incident_id = Uuid::new_v4();
    let resolved_started = Utc::now() - chrono::Duration::days(3);
    let resolved_at = resolved_started + chrono::Duration::minutes(45);

    sqlx::query(
        "INSERT INTO incidents (id, org_id, title, status, impact, started_at, resolved_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(resolved_incident_id)
    .bind(org_id)
    .bind("Elevated API latency")
    .bind(IncidentStatus::Resolved.to_string())
    .bind(IncidentImpact::Minor.to_string())
    .bind(resolved_started)
    .bind(resolved_at)
    .bind(user_id)
    .execute(&pool)
    .await?;

    // Link to API service
    sqlx::query(
        "INSERT INTO incident_services (incident_id, service_id) VALUES ($1, $2)",
    )
    .bind(resolved_incident_id)
    .bind(service_ids[0].0) // API
    .execute(&pool)
    .await?;

    // Add timeline updates for resolved incident
    let updates = vec![
        (resolved_started, IncidentStatus::Investigating, "We are investigating reports of elevated API response times."),
        (resolved_started + chrono::Duration::minutes(10), IncidentStatus::Identified, "Root cause identified: a slow database query in the authentication path."),
        (resolved_started + chrono::Duration::minutes(30), IncidentStatus::Monitoring, "Fix deployed. Monitoring for stability."),
        (resolved_started + chrono::Duration::minutes(45), IncidentStatus::Resolved, "API latency has returned to normal levels. The slow query has been optimized."),
    ];

    for (time, status, message) in &updates {
        sqlx::query(
            "INSERT INTO incident_updates (incident_id, status, message, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(resolved_incident_id)
        .bind(status.to_string())
        .bind(message)
        .bind(user_id)
        .bind(time)
        .execute(&pool)
        .await?;
    }
    println!("  Created resolved incident: Elevated API latency (with 4 updates)");

    // 6. Create an active incident
    let active_incident_id = Uuid::new_v4();
    let active_started = Utc::now() - chrono::Duration::hours(2);

    sqlx::query(
        "INSERT INTO incidents (id, org_id, title, status, impact, started_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(active_incident_id)
    .bind(org_id)
    .bind("Email delivery delays")
    .bind(IncidentStatus::Identified.to_string())
    .bind(IncidentImpact::Minor.to_string())
    .bind(active_started)
    .bind(user_id)
    .execute(&pool)
    .await?;

    // Link to Email Service
    sqlx::query(
        "INSERT INTO incident_services (incident_id, service_id) VALUES ($1, $2)",
    )
    .bind(active_incident_id)
    .bind(service_ids[4].0) // Email Service
    .execute(&pool)
    .await?;

    // Timeline updates for active incident
    sqlx::query(
        "INSERT INTO incident_updates (incident_id, status, message, created_by, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(active_incident_id)
    .bind(IncidentStatus::Investigating.to_string())
    .bind("We are investigating reports of delayed email delivery.")
    .bind(user_id)
    .bind(active_started)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO incident_updates (incident_id, status, message, created_by, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(active_incident_id)
    .bind(IncidentStatus::Identified.to_string())
    .bind("The issue has been identified as a backlog in our email provider's queue. We are working with them to resolve it.")
    .bind(user_id)
    .bind(active_started + chrono::Duration::minutes(30))
    .execute(&pool)
    .await?;

    println!("  Created active incident: Email delivery delays (with 2 updates)");

    // 7. Create monitors for services
    let monitors = vec![
        (service_ids[0].0, "http", serde_json::json!({"url": "https://api.example.com/health", "method": "GET", "expected_status": 200, "headers": {}})),
        (service_ids[1].0, "http", serde_json::json!({"url": "https://app.example.com", "method": "GET", "expected_status": 200, "headers": {}})),
        (service_ids[2].0, "tcp", serde_json::json!({"host": "db.example.com", "port": 5432})),
    ];

    for (service_id, monitor_type, config) in &monitors {
        sqlx::query(
            "INSERT INTO monitors (service_id, org_id, monitor_type, config, interval_seconds, timeout_ms, failure_threshold)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(service_id)
        .bind(org_id)
        .bind(monitor_type)
        .bind(config)
        .bind(60)
        .bind(10000)
        .bind(3)
        .execute(&pool)
        .await?;
    }
    println!("  Created 3 monitors (HTTP x2, TCP x1)");

    println!("\nSeed complete!");
    println!("  Organization: Demo Company (slug: demo)");
    println!("  Public page:  http://localhost:3000/s/demo");
    println!("  Dashboard:    http://localhost:3000/dashboard/demo");
    println!("\n  Note: To log in, you'll need to authenticate via GitHub OAuth.");
    println!("  The seed user (demo@statuspage.sh) is for reference only.");

    Ok(())
}
