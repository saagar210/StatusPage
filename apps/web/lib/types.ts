// --- Enums ---

export type ServiceStatus =
  | "operational"
  | "degraded_performance"
  | "partial_outage"
  | "major_outage"
  | "under_maintenance";

export type IncidentStatus =
  | "investigating"
  | "identified"
  | "monitoring"
  | "resolved";

export type IncidentImpact = "none" | "minor" | "major" | "critical";

export type MemberRole = "owner" | "admin" | "member";

export type MonitorType = "http" | "tcp" | "dns" | "ping";

export type CheckStatus = "success" | "failure" | "timeout";

export type OrganizationPlan = "free" | "pro" | "team";

export type SubscriptionStatus =
  | "inactive"
  | "checkout_pending"
  | "trialing"
  | "active"
  | "past_due"
  | "canceled"
  | "unpaid"
  | "incomplete"
  | "incomplete_expired";

export type DowngradeState =
  | "none"
  | "pending_customer_action"
  | "ready_to_enforce"
  | "enforced"
  | "canceled";

export type CustomDomainStatus =
  | "not_configured"
  | "pending_verification"
  | "verified"
  | "blocked_by_plan";

export type DisabledReason = "plan_limit";

export type InvitationDeliveryStatus =
  | "pending"
  | "accepted"
  | "expired"
  | "canceled"
  | "delivery_failed";

// --- Models ---

export interface Organization {
  id: string;
  name: string;
  slug: string;
  plan: OrganizationPlan;
  logo_url: string | null;
  brand_color: string;
  timezone: string;
  custom_domain: string | null;
  custom_domain_verified_at: string | null;
  custom_domain_status: CustomDomainStatus;
  stripe_customer_id: string | null;
  stripe_subscription_id: string | null;
  subscription_status: SubscriptionStatus;
  stripe_price_id: string | null;
  current_period_end: string | null;
  cancel_at_period_end: boolean;
  billing_email: string | null;
  trial_ends_at: string | null;
  downgrade_target_plan: OrganizationPlan | null;
  downgrade_started_at: string | null;
  downgrade_grace_ends_at: string | null;
  downgrade_state: DowngradeState;
  downgrade_warning_stage: number;
  created_at: string;
  updated_at: string;
}

export interface Service {
  id: string;
  org_id: string;
  name: string;
  description: string | null;
  current_status: ServiceStatus;
  display_order: number;
  group_name: string | null;
  is_visible: boolean;
  created_at: string;
  updated_at: string;
}

export interface Incident {
  id: string;
  org_id: string;
  title: string;
  status: IncidentStatus;
  impact: IncidentImpact;
  is_auto: boolean;
  started_at: string;
  resolved_at: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface IncidentUpdate {
  id: string;
  incident_id: string;
  status: IncidentStatus;
  message: string;
  created_by: string | null;
  created_at: string;
}

export interface IncidentWithDetails extends Incident {
  updates: IncidentUpdate[];
  affected_services: AffectedService[];
}

export interface AffectedService {
  service_id: string;
  service_name: string;
}

export interface Monitor {
  id: string;
  service_id: string;
  org_id: string;
  monitor_type: MonitorType;
  config: Record<string, unknown>;
  interval_seconds: number;
  timeout_ms: number;
  failure_threshold: number;
  is_active: boolean;
  disabled_reason: DisabledReason | null;
  consecutive_failures: number;
  last_checked_at: string | null;
  last_response_time_ms: number | null;
  created_at: string;
  updated_at: string;
}

export interface MonitorCheck {
  id: number;
  monitor_id: string;
  status: CheckStatus;
  response_time_ms: number | null;
  status_code: number | null;
  error_message: string | null;
  checked_at: string;
}

export interface NotificationPreferences {
  id: string;
  org_id: string;
  email_on_incident_created: boolean;
  email_on_incident_updated: boolean;
  email_on_incident_resolved: boolean;
  email_on_service_status_changed: boolean;
  webhook_on_incident_created: boolean;
  webhook_on_incident_updated: boolean;
  webhook_on_incident_resolved: boolean;
  webhook_on_service_status_changed: boolean;
  uptime_alert_threshold: number | null;
  uptime_alert_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface WebhookConfig {
  id: string;
  org_id: string;
  name: string;
  url: string;
  event_types: string[];
  is_enabled: boolean;
  disabled_reason: DisabledReason | null;
  created_at: string;
  updated_at: string;
}

export interface SubscriberListItem {
  id: string;
  email: string;
  is_verified: boolean;
  verification_sent_at: string | null;
  verified_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface NotificationLogEntry {
  id: string;
  notification_type: string;
  recipient_type: string;
  recipient_email: string;
  subject: string | null;
  status: string;
  error_message: string | null;
  attempt_count: number;
  max_attempts: number;
  sent_at: string | null;
  next_retry_at: string | null;
  created_at: string;
}

export interface WebhookDeliveryEntry {
  id: string;
  webhook_config_id: string;
  webhook_name: string;
  webhook_url: string;
  event_type: string;
  status: string;
  response_status_code: number | null;
  error_message: string | null;
  attempt_count: number;
  max_attempts: number;
  next_retry_at: string | null;
  delivered_at: string | null;
  created_at: string;
}

// --- Request DTOs ---

export interface CreateOrganizationRequest {
  name: string;
  slug?: string;
}

export interface UpdateOrganizationRequest {
  name?: string;
  slug?: string;
  brand_color?: string;
  timezone?: string;
  logo_url?: string | null;
  custom_domain?: string | null;
}

export interface MemberWithUser {
  id: string;
  org_id: string;
  user_id: string;
  role: MemberRole;
  created_at: string;
  user_name: string | null;
  user_email: string;
  user_image: string | null;
}

export interface Invitation {
  id: string;
  org_id: string;
  email: string;
  role: MemberRole;
  invited_by: string;
  token: string;
  expires_at: string;
  accepted_at: string | null;
  canceled_at: string | null;
  last_sent_at: string | null;
  created_at: string;
  updated_at: string;
  inviter_name: string | null;
  inviter_email: string;
  delivery_status: InvitationDeliveryStatus;
}

export interface CreateMemberRequest {
  email: string;
  role: MemberRole;
}

export interface CreateInvitationRequest {
  email: string;
  role: MemberRole;
}

export interface UpdateMemberRequest {
  role: MemberRole;
}

export interface BillingSummary {
  billing_enabled: boolean;
  portal_enabled: boolean;
  checkout_enabled: boolean;
  current_plan: OrganizationPlan;
  subscription_status: SubscriptionStatus;
  stripe_customer_id: string | null;
  billing_email: string | null;
  current_period_end: string | null;
  cancel_at_period_end: boolean;
  available_upgrades: OrganizationPlan[];
  entitlements: BillingEntitlements;
  downgrade_target_plan: OrganizationPlan | null;
  downgrade_started_at: string | null;
  downgrade_grace_ends_at: string | null;
  downgrade_state: DowngradeState;
  entitlement_violations: EntitlementViolation[];
  required_actions: string[];
  self_serve_downgrade: boolean;
}

export interface BillingEntitlements {
  max_monitors: number | null;
  custom_domain_enabled: boolean;
  outbound_webhooks_enabled: boolean;
  priority_support: boolean;
}

export interface EntitlementViolation {
  code: string;
  message: string;
  current_count: number | null;
  allowed_count: number | null;
}

export interface AuditLogEntry {
  id: string;
  org_id: string;
  actor_user_id: string | null;
  actor_type: string;
  action: string;
  target_type: string;
  target_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

export interface BillingEventEntry {
  stripe_event_id: string;
  event_type: string;
  processed_at: string;
}

export interface SupportQueueHealth {
  pending_email_deliveries: number;
  failed_email_deliveries: number;
  pending_webhook_deliveries: number;
  failed_webhook_deliveries: number;
  recent_billing_events?: number;
  pending_invitation_emails?: number;
  pending_downgrade_warnings?: number;
  organizations_in_grace?: number;
}

export interface SupportOrganizationSummary {
  id: string;
  name: string;
  slug: string;
  plan: OrganizationPlan;
  subscription_status: SubscriptionStatus;
  stripe_customer_id: string | null;
  stripe_subscription_id: string | null;
  billing_email: string | null;
  custom_domain: string | null;
  custom_domain_verified_at: string | null;
  custom_domain_status: CustomDomainStatus;
  downgrade_target_plan: OrganizationPlan | null;
  downgrade_grace_ends_at: string | null;
  downgrade_state: DowngradeState;
  member_count: number;
  pending_invitation_count: number;
  subscriber_count: number;
  webhook_count: number;
}

export interface SupportSearchResult {
  id: string;
  name: string;
  slug: string;
  plan: OrganizationPlan;
  subscription_status: SubscriptionStatus;
  billing_email: string | null;
  stripe_customer_id: string | null;
  stripe_subscription_id: string | null;
  downgrade_state: DowngradeState;
  downgrade_target_plan: OrganizationPlan | null;
  downgrade_grace_ends_at: string | null;
}

export interface SupportOrganizationPayload {
  organization: SupportOrganizationSummary;
  queue_health: SupportQueueHealth;
  entitlement_violations: EntitlementViolation[];
  required_actions: string[];
  invitations: Invitation[];
  recent_billing_events: BillingEventEntry[];
  failed_email_deliveries: NotificationLogEntry[];
  failed_webhook_deliveries: WebhookDeliveryEntry[];
  recent_audit_logs: AuditLogEntry[];
}

export interface ResolvedCustomDomain {
  slug: string;
  organization: {
    name: string;
    logo_url: string | null;
    brand_color: string;
  };
}

export interface CreateServiceRequest {
  name: string;
  description?: string;
  group_name?: string;
  is_visible?: boolean;
}

export interface UpdateServiceRequest {
  name?: string;
  description?: string;
  current_status?: ServiceStatus;
  group_name?: string;
  is_visible?: boolean;
}

export interface ReorderServicesRequest {
  service_ids: string[];
}

export interface CreateIncidentRequest {
  title: string;
  status?: IncidentStatus;
  impact: IncidentImpact;
  message: string;
  affected_service_ids: string[];
}

export interface UpdateIncidentRequest {
  title?: string;
  status?: IncidentStatus;
  impact?: IncidentImpact;
}

export interface CreateIncidentUpdateRequest {
  status: IncidentStatus;
  message: string;
}

export interface CreateMonitorRequest {
  service_id: string;
  monitor_type: MonitorType;
  config: Record<string, unknown>;
  interval_seconds?: number;
  timeout_ms?: number;
  failure_threshold?: number;
}

export interface UpdateMonitorRequest {
  config?: Record<string, unknown>;
  interval_seconds?: number;
  timeout_ms?: number;
  failure_threshold?: number;
  is_active?: boolean;
}

export interface UpdateNotificationPreferencesRequest {
  email_on_incident_created?: boolean;
  email_on_incident_updated?: boolean;
  email_on_incident_resolved?: boolean;
  email_on_service_status_changed?: boolean;
  webhook_on_incident_created?: boolean;
  webhook_on_incident_updated?: boolean;
  webhook_on_incident_resolved?: boolean;
  webhook_on_service_status_changed?: boolean;
  uptime_alert_threshold?: number;
  uptime_alert_enabled?: boolean;
}

export interface CreateWebhookConfigRequest {
  name: string;
  url: string;
  secret: string;
  event_types: string[];
  is_enabled?: boolean;
}

export interface UpdateWebhookConfigRequest {
  name?: string;
  url?: string;
  secret?: string;
  event_types?: string[];
  is_enabled?: boolean;
}

// --- API Response shapes ---

export interface ApiResponse<T> {
  data: T;
}

export interface ApiListResponse<T> {
  data: T[];
  pagination: {
    page: number;
    per_page: number;
    total: number;
  };
}

export interface ApiError {
  error: {
    code: string;
    message: string;
  };
}

// --- Public API types ---

export interface PublicStatusResponse {
  organization: {
    name: string;
    logo_url: string | null;
    brand_color: string;
  };
  overall_status: ServiceStatus;
  services: PublicService[];
  active_incidents: PublicIncident[];
}

export interface PublicMessageResponse {
  message: string;
}

export interface PublicService {
  id: string;
  name: string;
  current_status: ServiceStatus;
  group_name: string | null;
}

export interface PublicIncident {
  id: string;
  title: string;
  status: IncidentStatus;
  impact: IncidentImpact;
  started_at: string;
  resolved_at: string | null;
  updates: IncidentUpdate[];
  affected_services: string[];
}

export interface UptimeResponse {
  services: ServiceUptime[];
}

export interface ServiceUptime {
  service_id: string;
  service_name: string;
  days: UptimeDay[];
  overall_uptime: number | null;
}

export interface UptimeDay {
  date: string;
  uptime_percentage: number | null;
  avg_response_time_ms: number | null;
}

// --- Display helpers ---

export const SERVICE_STATUS_LABELS: Record<ServiceStatus, string> = {
  operational: "Operational",
  degraded_performance: "Degraded Performance",
  partial_outage: "Partial Outage",
  major_outage: "Major Outage",
  under_maintenance: "Under Maintenance",
};

export const INCIDENT_STATUS_LABELS: Record<IncidentStatus, string> = {
  investigating: "Investigating",
  identified: "Identified",
  monitoring: "Monitoring",
  resolved: "Resolved",
};

export const INCIDENT_IMPACT_LABELS: Record<IncidentImpact, string> = {
  none: "None",
  minor: "Minor",
  major: "Major",
  critical: "Critical",
};

export const INCIDENT_IMPACT_COLORS: Record<IncidentImpact, string> = {
  none: "bg-gray-100 text-gray-800",
  minor: "bg-yellow-100 text-yellow-800",
  major: "bg-orange-100 text-orange-800",
  critical: "bg-red-100 text-red-800",
};


export const PLAN_MONITOR_LIMITS: Record<OrganizationPlan, number | null> = {
  free: 3,
  pro: 20,
  team: null,
};

export const PLAN_FEATURES: Record<OrganizationPlan, BillingEntitlements> = {
  free: {
    max_monitors: 3,
    custom_domain_enabled: false,
    outbound_webhooks_enabled: false,
    priority_support: false,
  },
  pro: {
    max_monitors: 20,
    custom_domain_enabled: true,
    outbound_webhooks_enabled: true,
    priority_support: false,
  },
  team: {
    max_monitors: null,
    custom_domain_enabled: true,
    outbound_webhooks_enabled: true,
    priority_support: true,
  },
};

export function formatPlanMonitorLimit(plan: OrganizationPlan): string {
  const limit = PLAN_MONITOR_LIMITS[plan];
  return limit === null ? "Unlimited" : String(limit);
}

export function formatOrganizationPlan(plan: OrganizationPlan): string {
  return plan.charAt(0).toUpperCase() + plan.slice(1);
}

export function formatSubscriptionStatus(status: SubscriptionStatus): string {
  return status
    .split("_")
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

export function formatDowngradeState(state: DowngradeState | null | undefined): string {
  return (state || "none")
    .split("_")
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}
