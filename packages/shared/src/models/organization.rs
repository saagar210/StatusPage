use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{CustomDomainStatus, DowngradeState, OrganizationPlan, SubscriptionStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingEntitlements {
    pub max_monitors: Option<i64>,
    pub custom_domain_enabled: bool,
    pub outbound_webhooks_enabled: bool,
    pub priority_support: bool,
}

impl From<OrganizationPlan> for BillingEntitlements {
    fn from(plan: OrganizationPlan) -> Self {
        Self {
            max_monitors: plan.max_monitors(),
            custom_domain_enabled: plan.allows_custom_domain(),
            outbound_webhooks_enabled: plan.allows_outbound_webhooks(),
            priority_support: plan.has_priority_support(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementViolation {
    pub code: String,
    pub message: String,
    pub current_count: Option<i64>,
    pub allowed_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub plan: OrganizationPlan,
    pub logo_url: Option<String>,
    pub brand_color: String,
    pub timezone: String,
    pub custom_domain: Option<String>,
    pub custom_domain_verified_at: Option<DateTime<Utc>>,
    pub custom_domain_status: CustomDomainStatus,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub subscription_status: SubscriptionStatus,
    pub stripe_price_id: Option<String>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub billing_email: Option<String>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub downgrade_target_plan: Option<OrganizationPlan>,
    pub downgrade_started_at: Option<DateTime<Utc>>,
    pub downgrade_grace_ends_at: Option<DateTime<Utc>>,
    pub downgrade_state: DowngradeState,
    pub downgrade_warning_stage: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub brand_color: Option<String>,
    pub timezone: Option<String>,
    pub logo_url: Option<String>,
    pub custom_domain: Option<String>,
}
