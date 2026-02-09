import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { PublicIncident, IncidentStatus } from "@/lib/types";
import { INCIDENT_STATUS_LABELS } from "@/lib/types";

const statusDotColors: Record<IncidentStatus, string> = {
  investigating: "bg-red-500",
  identified: "bg-orange-500",
  monitoring: "bg-yellow-500",
  resolved: "bg-green-500",
};

export function ActiveIncidents({
  incidents,
}: {
  incidents: PublicIncident[];
}) {
  if (incidents.length === 0) return null;

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold">Active Incidents</h2>
      {incidents.map((incident) => (
        <Card key={incident.id}>
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="text-base">{incident.title}</CardTitle>
              <Badge variant="outline">
                {INCIDENT_STATUS_LABELS[incident.status]}
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground">
              Affecting: {incident.affected_services.join(", ")}
            </p>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {incident.updates.map((update) => (
                <div key={update.id} className="flex gap-3">
                  <div
                    className={`mt-1.5 h-2.5 w-2.5 flex-shrink-0 rounded-full ${statusDotColors[update.status]}`}
                  />
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-medium">
                        {INCIDENT_STATUS_LABELS[update.status]}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {new Date(update.created_at).toLocaleString()}
                      </span>
                    </div>
                    <p className="mt-1 text-sm">{update.message}</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
