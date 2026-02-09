use std::time::{Duration, Instant};

use shared::enums::CheckStatus;
use shared::models::monitor::TcpConfig;
use tokio::net::TcpStream;

use super::{CheckResult, Checker};

pub struct TcpChecker {
    config: TcpConfig,
}

impl TcpChecker {
    pub fn new(config: TcpConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Checker for TcpChecker {
    async fn check(&self, timeout: Duration) -> CheckResult {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let start = Instant::now();

        match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_stream)) => {
                let elapsed = start.elapsed().as_millis() as u32;
                CheckResult {
                    status: CheckStatus::Success,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: None,
                }
            }
            Ok(Err(e)) => {
                let elapsed = start.elapsed().as_millis() as u32;
                CheckResult {
                    status: CheckStatus::Failure,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some(format!("Connection failed: {}", e)),
                }
            }
            Err(_) => {
                let elapsed = start.elapsed().as_millis() as u32;
                CheckResult {
                    status: CheckStatus::Timeout,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some("Connection timed out".to_string()),
                }
            }
        }
    }
}
