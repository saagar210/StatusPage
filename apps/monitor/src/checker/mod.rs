pub mod dns;
pub mod http;
#[cfg(feature = "ping")]
pub mod ping;
pub mod tcp;

use std::time::Duration;

use shared::enums::CheckStatus;
use shared::models::monitor::MonitorConfig;

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: CheckStatus,
    pub response_time_ms: u32,
    pub status_code: Option<u16>,
    pub error_message: Option<String>,
}

#[async_trait::async_trait]
pub trait Checker: Send + Sync {
    async fn check(&self, timeout: Duration) -> CheckResult;
}

pub fn create_checker(config: &MonitorConfig) -> Box<dyn Checker> {
    match config {
        MonitorConfig::Http(c) => Box::new(http::HttpChecker::new(c.clone())),
        MonitorConfig::Tcp(c) => Box::new(tcp::TcpChecker::new(c.clone())),
        MonitorConfig::Dns(c) => Box::new(dns::DnsChecker::new(c.clone())),
        #[cfg(feature = "ping")]
        MonitorConfig::Ping(c) => Box::new(ping::PingChecker::new(c.clone())),
        #[cfg(not(feature = "ping"))]
        MonitorConfig::Ping(_) => Box::new(UnsupportedChecker("Ping checker not compiled (enable 'ping' feature)".to_string())),
    }
}

#[cfg(not(feature = "ping"))]
struct UnsupportedChecker(String);

#[cfg(not(feature = "ping"))]
#[async_trait::async_trait]
impl Checker for UnsupportedChecker {
    async fn check(&self, _timeout: Duration) -> CheckResult {
        CheckResult {
            status: CheckStatus::Failure,
            response_time_ms: 0,
            status_code: None,
            error_message: Some(self.0.clone()),
        }
    }
}
