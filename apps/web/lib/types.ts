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

// --- Models ---

export interface Organization {
  id: string;
  name: string;
  slug: string;
  plan: string;
  logo_url: string | null;
  brand_color: string;
  timezone: string;
  custom_domain: string | null;
  stripe_customer_id: string | null;
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
  logo_url?: string;
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
