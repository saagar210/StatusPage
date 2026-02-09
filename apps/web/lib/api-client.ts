import { cookies } from "next/headers";
import type {
  ApiResponse,
  ApiListResponse,
  Organization,
  CreateOrganizationRequest,
  UpdateOrganizationRequest,
  Service,
  CreateServiceRequest,
  UpdateServiceRequest,
  Incident,
  IncidentWithDetails,
  CreateIncidentRequest,
  UpdateIncidentRequest,
  CreateIncidentUpdateRequest,
  IncidentUpdate,
  Monitor,
  CreateMonitorRequest,
  UpdateMonitorRequest,
  MonitorCheck,
  PublicStatusResponse,
  PublicIncident,
  UptimeResponse,
} from "./types";

const INTERNAL_API_URL =
  process.env.INTERNAL_API_URL || "http://localhost:4000";

class ApiClientError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiClientError";
  }
}

async function getSessionCookie(): Promise<string> {
  const cookieStore = await cookies();
  const sessionToken =
    cookieStore.get("authjs.session-token")?.value ||
    cookieStore.get("__Secure-authjs.session-token")?.value;
  if (!sessionToken) return "";
  return `authjs.session-token=${sessionToken}`;
}

async function fetchApi<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const cookie = await getSessionCookie();
  const url = `${INTERNAL_API_URL}${path}`;

  const res = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      Cookie: cookie,
      ...options.headers,
    },
    cache: "no-store",
  });

  if (!res.ok) {
    const errorBody = await res.json().catch(() => ({
      error: { code: "UNKNOWN", message: "Request failed" },
    }));
    throw new ApiClientError(
      res.status,
      errorBody.error?.code || "UNKNOWN",
      errorBody.error?.message || "Request failed",
    );
  }

  return res.json() as Promise<T>;
}

// --- Organizations ---

export async function getOrganizations(): Promise<Organization[]> {
  const res = await fetchApi<ApiResponse<Organization[]>>(
    "/api/organizations",
  );
  return res.data;
}

export async function createOrganization(
  data: CreateOrganizationRequest,
): Promise<Organization> {
  const res = await fetchApi<ApiResponse<Organization>>(
    "/api/organizations",
    {
      method: "POST",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function getOrganization(slug: string): Promise<Organization> {
  const res = await fetchApi<ApiResponse<Organization>>(
    `/api/organizations/${slug}`,
  );
  return res.data;
}

export async function updateOrganization(
  slug: string,
  data: UpdateOrganizationRequest,
): Promise<Organization> {
  const res = await fetchApi<ApiResponse<Organization>>(
    `/api/organizations/${slug}`,
    {
      method: "PATCH",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

// --- Services ---

export async function getServices(slug: string): Promise<Service[]> {
  const res = await fetchApi<ApiResponse<Service[]>>(
    `/api/organizations/${slug}/services`,
  );
  return res.data;
}

export async function createService(
  slug: string,
  data: CreateServiceRequest,
): Promise<Service> {
  const res = await fetchApi<ApiResponse<Service>>(
    `/api/organizations/${slug}/services`,
    {
      method: "POST",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function updateService(
  slug: string,
  serviceId: string,
  data: UpdateServiceRequest,
): Promise<Service> {
  const res = await fetchApi<ApiResponse<Service>>(
    `/api/organizations/${slug}/services/${serviceId}`,
    {
      method: "PATCH",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function deleteService(
  slug: string,
  serviceId: string,
): Promise<void> {
  await fetchApi(`/api/organizations/${slug}/services/${serviceId}`, {
    method: "DELETE",
  });
}

export async function reorderServices(
  slug: string,
  serviceIds: string[],
): Promise<void> {
  await fetchApi(`/api/organizations/${slug}/services/reorder`, {
    method: "PATCH",
    body: JSON.stringify({ service_ids: serviceIds }),
  });
}

// --- Incidents ---

export async function getIncidents(
  slug: string,
  params?: { status?: string; page?: number; per_page?: number },
): Promise<ApiListResponse<Incident>> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set("status", params.status);
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.per_page) searchParams.set("per_page", String(params.per_page));
  const query = searchParams.toString() ? `?${searchParams.toString()}` : "";

  return fetchApi<ApiListResponse<Incident>>(
    `/api/organizations/${slug}/incidents${query}`,
  );
}

export async function getIncident(
  slug: string,
  incidentId: string,
): Promise<IncidentWithDetails> {
  const res = await fetchApi<ApiResponse<IncidentWithDetails>>(
    `/api/organizations/${slug}/incidents/${incidentId}`,
  );
  return res.data;
}

export async function createIncident(
  slug: string,
  data: CreateIncidentRequest,
): Promise<Incident> {
  const res = await fetchApi<ApiResponse<Incident>>(
    `/api/organizations/${slug}/incidents`,
    {
      method: "POST",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function updateIncident(
  slug: string,
  incidentId: string,
  data: UpdateIncidentRequest,
): Promise<Incident> {
  const res = await fetchApi<ApiResponse<Incident>>(
    `/api/organizations/${slug}/incidents/${incidentId}`,
    {
      method: "PATCH",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function createIncidentUpdate(
  slug: string,
  incidentId: string,
  data: CreateIncidentUpdateRequest,
): Promise<IncidentUpdate> {
  const res = await fetchApi<ApiResponse<IncidentUpdate>>(
    `/api/organizations/${slug}/incidents/${incidentId}/updates`,
    {
      method: "POST",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function deleteIncident(
  slug: string,
  incidentId: string,
): Promise<void> {
  await fetchApi(`/api/organizations/${slug}/incidents/${incidentId}`, {
    method: "DELETE",
  });
}

// --- Monitors ---

export async function getMonitors(slug: string): Promise<Monitor[]> {
  const res = await fetchApi<ApiResponse<Monitor[]>>(
    `/api/organizations/${slug}/monitors`,
  );
  return res.data;
}

export async function getMonitor(
  slug: string,
  monitorId: string,
): Promise<Monitor> {
  const res = await fetchApi<ApiResponse<Monitor>>(
    `/api/organizations/${slug}/monitors/${monitorId}`,
  );
  return res.data;
}

export async function createMonitor(
  slug: string,
  data: CreateMonitorRequest,
): Promise<Monitor> {
  const res = await fetchApi<ApiResponse<Monitor>>(
    `/api/organizations/${slug}/monitors`,
    {
      method: "POST",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function updateMonitor(
  slug: string,
  monitorId: string,
  data: UpdateMonitorRequest,
): Promise<Monitor> {
  const res = await fetchApi<ApiResponse<Monitor>>(
    `/api/organizations/${slug}/monitors/${monitorId}`,
    {
      method: "PATCH",
      body: JSON.stringify(data),
    },
  );
  return res.data;
}

export async function deleteMonitor(
  slug: string,
  monitorId: string,
): Promise<void> {
  await fetchApi(`/api/organizations/${slug}/monitors/${monitorId}`, {
    method: "DELETE",
  });
}

export async function getMonitorChecks(
  slug: string,
  monitorId: string,
  params?: { page?: number; per_page?: number },
): Promise<ApiListResponse<MonitorCheck>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.per_page) searchParams.set("per_page", String(params.per_page));
  const query = searchParams.toString() ? `?${searchParams.toString()}` : "";

  return fetchApi<ApiListResponse<MonitorCheck>>(
    `/api/organizations/${slug}/monitors/${monitorId}/checks${query}`,
  );
}

// --- Public API ---

export async function getPublicStatus(
  slug: string,
): Promise<PublicStatusResponse> {
  const res = await fetchApi<ApiResponse<PublicStatusResponse>>(
    `/api/public/${slug}/status`,
  );
  return res.data;
}

export async function getPublicUptime(slug: string): Promise<UptimeResponse> {
  const res = await fetchApi<ApiResponse<UptimeResponse>>(
    `/api/public/${slug}/uptime`,
  );
  return res.data;
}

export async function getPublicIncidents(
  slug: string,
  page = 1,
  perPage = 20,
): Promise<ApiListResponse<PublicIncident>> {
  return fetchApi<ApiListResponse<PublicIncident>>(
    `/api/public/${slug}/incidents?page=${page}&per_page=${perPage}`,
  );
}

export { ApiClientError };
