import { getServices, getIncidents } from "@/lib/api-client";
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

  try {
    services = await getServices(slug);
    const incidentRes = await getIncidents(slug, { per_page: 5 });
    activeIncidents = incidentRes.data.filter((i) => i.status !== "resolved");
  } catch {
    // API might not be available
  }

  const operational = services.filter((s) => s.current_status === "operational").length;

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
            <p className="text-xs text-muted-foreground">
              {operational} operational
            </p>
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

      {services.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Services</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {services.map((service) => (
                <div
                  key={service.id}
                  className="flex items-center justify-between"
                >
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
