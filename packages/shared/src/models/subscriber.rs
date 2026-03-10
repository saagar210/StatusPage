use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscriber {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub verification_sent_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
    pub unsubscribe_token: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub email: String,
}
