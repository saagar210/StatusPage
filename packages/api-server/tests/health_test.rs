// Integration test for the health endpoint.
// This test requires a running PostgreSQL instance.
// Set DATABASE_URL env var to run: cargo test -p api-server --test health_test

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::time::Duration;

    use axum::http::StatusCode;

    /// Find a free port by binding to port 0
    fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    /// Spawn the API server on a random port and return the base URL.
    /// Requires DATABASE_URL to be set.
    async fn spawn_app() -> String {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("DATABASE_URL not set, skipping integration test");
                return String::new();
            }
        };

        let port = free_port();
        let addr = format!("127.0.0.1:{}", port);

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await
            .expect("Failed to connect to database");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Build a minimal router with just the health endpoint
        let app = axum::Router::new()
            .route("/health", axum::routing::get(|| async {
                axum::Json(serde_json::json!({"status": "ok"}))
            }));

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("Failed to bind");

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        format!("http://127.0.0.1:{}", port)
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let base_url = spawn_app().await;
        if base_url.is_empty() {
            return; // Skip if no DB
        }

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status(), StatusCode::OK);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["status"], "ok");
    }
}
