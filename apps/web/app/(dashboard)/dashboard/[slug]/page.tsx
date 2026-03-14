import {
  getBillingSummary,
  getIncidents,
  getInvitations,
  getMonitors,
  getServices,
} from "@/lib/api-client";
import { buildSetupChecklist } from "@/lib/onboarding";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { StatusBadge } from "@/components/dashboard/status-badge";
import type { ServiceStatus, IncidentStatus } from "@/lib/types";

export default async function OrgOverview({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  let services: { id: string; name: string; current_status: ServiceStatus }[] = [];
  let activeIncidents: { id: string; title: string; status: IncidentStatus }[] = [];
  let monitorCount = 0;
  let invitationCount = 0;
  let billingReviewed = false;

  try {
    const [serviceData, incidentRes, monitors, invitations, billing] = await Promise.all([
      getServices(slug),
      getIncidents(slug, { per_page: 5 }),
      getMonitors(slug),
      getInvitations(slug),
      getBillingSummary(slug),
    ]);
    services = serviceData;
    activeIncidents = incidentRes.data.filter((i) => i.status !== "resolved");
    monitorCount = monitors.filter((monitor) => !monitor.disabled_reason).length;
    invitationCount = invitations.filter((invite) => invite.delivery_status === "pending").length;
    billingReviewed =
      billing.current_plan !== "free" || billing.downgrade_state === "canceled";
  } catch {
    // API might not be available
  }

  const operational = services.filter((s) => s.current_status === "operational").length;
  const checklist = buildSetupChecklist({
    hasService: services.length > 0,
    hasMonitor: monitorCount > 0,
    hasSubscriberOrInvite: invitationCount > 0,
    hasCustomDomain: false,
    upgradedPlan: billingReviewed,
  });

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Overview</h1>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Services
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{services.length}</div>
            <p className="text-xs text-muted-foreground">{operational} operational</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Active Incidents
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{activeIncidents.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Status
            </CardTitle>
          </CardHeader>
          <CardContent>
            {activeIncidents.length === 0 ? (
              <Badge variant="default" className="bg-green-500">
                All Operational
              </Badge>
            ) : (
              <Badge variant="destructive">Issues Detected</Badge>
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Managed Setup Checklist</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-2">
          {checklist.map((item) => (
            <div
              key={item.key}
              className="flex items-center justify-between rounded-lg border p-4"
            >
              <div>
                <p className="font-medium">{item.label}</p>
                <p className="text-xs text-muted-foreground">
                  {item.optional ? "Optional" : "Required for a healthy managed launch"}
                </p>
              </div>
              <Badge variant={item.complete ? "secondary" : "outline"}>
                {item.complete ? "Complete" : item.optional ? "Optional" : "Pending"}
              </Badge>
            </div>
          ))}
        </CardContent>
      </Card>

      {services.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Services</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {services.map((service) => (
                <div key={service.id} className="flex items-center justify-between">
                  <span className="font-medium">{service.name}</span>
                  <StatusBadge status={service.current_status} />
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
