use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::enums::{CheckStatus, MonitorType};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Monitor {
    pub id: Uuid,
    pub service_id: Uuid,
    pub org_id: Uuid,
    pub monitor_type: MonitorType,
    pub config: serde_json::Value,
    pub interval_seconds: i32,
    pub timeout_ms: i32,
    pub failure_threshold: i32,
    pub is_active: bool,
    pub consecutive_failures: i32,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub last_response_time_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MonitorCheck {
    pub id: i64,
    pub monitor_id: Uuid,
    pub status: CheckStatus,
    pub response_time_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeDaily {
    pub monitor_id: Uuid,
    pub date: chrono::NaiveDate,
    pub total_checks: i32,
    pub successful_checks: i32,
    pub avg_response_time_ms: Option<f64>,
    pub min_response_time_ms: Option<i32>,
    pub max_response_time_ms: Option<i32>,
    pub uptime_percentage: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonitorConfig {
    Http(HttpConfig),
    Tcp(TcpConfig),
    Dns(DnsConfig),
    Ping(PingConfig),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpConfig {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_expected_status")]
    pub expected_status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub keyword: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_expected_status() -> u16 {
    200
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TcpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DnsConfig {
    pub hostname: String,
    pub expected_ip: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingConfig {
    pub host: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMonitorRequest {
    pub service_id: Uuid,
    pub monitor_type: MonitorType,
    pub config: serde_json::Value,
    pub interval_seconds: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub failure_threshold: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMonitorRequest {
    pub config: Option<serde_json::Value>,
    pub interval_seconds: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub failure_threshold: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorWithLatestCheck {
    #[serde(flatten)]
    pub monitor: Monitor,
    pub latest_check_status: Option<CheckStatus>,
    pub uptime_percentage: Option<f64>,
    pub service_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseTimePoint {
    pub timestamp: DateTime<Utc>,
    pub avg_response_time_ms: Option<f64>,
    pub status: Option<CheckStatus>,
}
