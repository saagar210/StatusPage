use std::time::{Duration, Instant};

use shared::enums::CheckStatus;
use shared::models::monitor::HttpConfig;

use super::{CheckResult, Checker};

pub struct HttpChecker {
    config: HttpConfig,
}

impl HttpChecker {
    pub fn new(config: HttpConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Checker for HttpChecker {
    async fn check(&self, timeout: Duration) -> CheckResult {
        let client = match reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent("StatusPage.sh Monitor/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return CheckResult {
                    status: CheckStatus::Failure,
                    response_time_ms: 0,
                    status_code: None,
                    error_message: Some(format!("Failed to create HTTP client: {}", e)),
                };
            }
        };

        let method = match self.config.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "HEAD" => reqwest::Method::HEAD,
            "POST" => reqwest::Method::POST,
            _ => reqwest::Method::GET,
        };

        let mut request = client.request(method, &self.config.url);

        for (key, value) in &self.config.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        if let Some(ref body) = self.config.body {
            request = request.body(body.clone());
        }

        let start = Instant::now();

        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                let elapsed = start.elapsed().as_millis() as u32;
                let status = if e.is_timeout() {
                    CheckStatus::Timeout
                } else {
                    CheckStatus::Failure
                };
                return CheckResult {
                    status,
                    response_time_ms: elapsed,
                    status_code: None,
                    error_message: Some(format!("{}", e)),
                };
            }
        };

        let elapsed = start.elapsed().as_millis() as u32;
        let status_code = response.status().as_u16();

        // Check status code
        if status_code != self.config.expected_status {
            return CheckResult {
                status: CheckStatus::Failure,
                response_time_ms: elapsed,
                status_code: Some(status_code),
                error_message: Some(format!(
                    "Expected status {}, got {}",
                    self.config.expected_status, status_code
                )),
            };
        }

        // Check keyword if configured
        if let Some(ref keyword) = self.config.keyword {
            match response.text().await {
                Ok(body) => {
                    if !body.contains(keyword.as_str()) {
                        return CheckResult {
                            status: CheckStatus::Failure,
                            response_time_ms: elapsed,
                            status_code: Some(status_code),
                            error_message: Some(format!(
                                "Keyword '{}' not found in response body",
                                keyword
                            )),
                        };
                    }
                }
                Err(e) => {
                    return CheckResult {
                        status: CheckStatus::Failure,
                        response_time_ms: elapsed,
                        status_code: Some(status_code),
                        error_message: Some(format!("Failed to read response body: {}", e)),
                    };
                }
            }
        }

        CheckResult {
            status: CheckStatus::Success,
            response_time_ms: elapsed,
            status_code: Some(status_code),
            error_message: None,
        }
    }
}
