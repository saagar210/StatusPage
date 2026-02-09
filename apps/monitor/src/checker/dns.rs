use std::time::{Duration, Instant};

use hickory_resolver::TokioAsyncResolver;
use shared::enums::CheckStatus;
use shared::models::monitor::DnsConfig;

use super::{CheckResult, Checker};

pub struct DnsChecker {
    config: DnsConfig,
}

impl DnsChecker {
    pub fn new(config: DnsConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Checker for DnsChecker {
    async fn check(&self, timeout: Duration) -> CheckResult {
        let start = Instant::now();

        let resolver = match TokioAsyncResolver::tokio_from_system_conf() {
            Ok(r) => r,
            Err(e) => {
                return CheckResult {
                    status: CheckStatus::Failure,
                    response_time_ms: 0,
                    status_code: None,
                    error_message: Some(format!("Failed to create DNS resolver: {}", e)),
                };
            }
        };

        let result = match tokio::time::timeout(timeout, resolver.lookup_ip(&self.config.hostname))
            .await
        {
            Ok(Ok(lookup)) => lookup,
            Ok(Err(e)) => {
                let elapsed = start.elapsed().as_millis() as u32;
                return CheckResult {
                    status: CheckStatus::Failure,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some(format!("DNS lookup failed: {}", e)),
                };
            }
            Err(_) => {
                let elapsed = start.elapsed().as_millis() as u32;
                return CheckResult {
                    status: CheckStatus::Timeout,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some("DNS lookup timed out".to_string()),
                };
            }
        };

        let elapsed = start.elapsed().as_millis() as u32;

        // Check expected IP if configured
        if let Some(ref expected_ip) = self.config.expected_ip {
            let ips: Vec<String> = result.iter().map(|ip| ip.to_string()).collect();
            if !ips.contains(expected_ip) {
                return CheckResult {
                    status: CheckStatus::Failure,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some(format!(
                        "Expected IP {} not found in results: {:?}",
                        expected_ip, ips
                    )),
                };
            }
        }

        CheckResult {
            status: CheckStatus::Success,
            response_time_ms: elapsed,
            status_code: None,
            error_message: None,
        }
    }
}
