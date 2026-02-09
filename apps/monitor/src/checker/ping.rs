use std::net::IpAddr;
use std::time::{Duration, Instant};

use shared::enums::CheckStatus;
use shared::models::monitor::PingConfig;
use surge_ping::{Client, PingIdentifier, PingSequence};

use super::{CheckResult, Checker};

pub struct PingChecker {
    config: PingConfig,
}

impl PingChecker {
    pub fn new(config: PingConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Checker for PingChecker {
    async fn check(&self, timeout: Duration) -> CheckResult {
        let start = Instant::now();

        // Resolve host to IP
        let addr: IpAddr = match self.config.host.parse() {
            Ok(ip) => ip,
            Err(_) => {
                // Try DNS resolution
                match tokio::net::lookup_host(format!("{}:0", self.config.host)).await {
                    Ok(mut addrs) => match addrs.next() {
                        Some(addr) => addr.ip(),
                        None => {
                            return CheckResult {
                                status: CheckStatus::Failure,
                                response_time_ms: 0,
                                status_code: None,
                                error_message: Some(format!(
                                    "Could not resolve host: {}",
                                    self.config.host
                                )),
                            };
                        }
                    },
                    Err(e) => {
                        return CheckResult {
                            status: CheckStatus::Failure,
                            response_time_ms: 0,
                            status_code: None,
                            error_message: Some(format!("DNS resolution failed: {}", e)),
                        };
                    }
                }
            }
        };

        let client = match Client::new(&surge_ping::Config::default()) {
            Ok(c) => c,
            Err(e) => {
                return CheckResult {
                    status: CheckStatus::Failure,
                    response_time_ms: 0,
                    status_code: None,
                    error_message: Some(format!(
                        "Failed to create ping client (may need CAP_NET_RAW): {}",
                        e
                    )),
                };
            }
        };

        let payload = [0u8; 56];
        let mut pinger = client.pinger(addr, PingIdentifier(rand::random())).await;
        pinger.timeout(timeout);

        match pinger.ping(PingSequence(0), &payload).await {
            Ok((_reply, rtt)) => {
                let elapsed = start.elapsed().as_millis() as u32;
                let _ = rtt; // We use our own timing
                CheckResult {
                    status: CheckStatus::Success,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: None,
                }
            }
            Err(e) => {
                let elapsed = start.elapsed().as_millis() as u32;
                let status = if elapsed >= timeout.as_millis() as u32 {
                    CheckStatus::Timeout
                } else {
                    CheckStatus::Failure
                };
                CheckResult {
                    status,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some(format!("Ping failed: {}", e)),
                }
            }
        }
    }
}
