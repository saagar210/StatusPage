import { cn } from "@/lib/utils";
import type { PublicService, ServiceStatus } from "@/lib/types";
import { SERVICE_STATUS_LABELS } from "@/lib/types";
import { CheckCircle, AlertTriangle, XCircle, Wrench } from "lucide-react";

const statusIcons: Record<ServiceStatus, React.ElementType> = {
  operational: CheckCircle,
  degraded_performance: AlertTriangle,
  partial_outage: AlertTriangle,
  major_outage: XCircle,
  under_maintenance: Wrench,
};

const statusColors: Record<ServiceStatus, string> = {
  operational: "text-green-500",
  degraded_performance: "text-yellow-500",
  partial_outage: "text-orange-500",
  major_outage: "text-red-500",
  under_maintenance: "text-blue-500",
};

export function ServiceList({ services }: { services: PublicService[] }) {
  // Group services by group_name
  const grouped = new Map<string | null, PublicService[]>();
  for (const service of services) {
    const group = service.group_name;
    const existing = grouped.get(group) || [];
    existing.push(service);
    grouped.set(group, existing);
  }

  return (
    <div className="space-y-4">
      {Array.from(grouped.entries()).map(([group, groupServices]) => (
        <div key={group || "ungrouped"}>
          {group && (
            <h3 className="mb-2 text-sm font-medium text-muted-foreground">
              {group}
            </h3>
          )}
          <div className="rounded-lg border divide-y">
            {groupServices.map((service) => {
              const Icon = statusIcons[service.current_status];
              return (
                <div
                  key={service.id}
                  className="flex items-center justify-between px-4 py-3"
                >
                  <span className="font-medium">{service.name}</span>
                  <div
                    className={cn(
                      "flex items-center gap-2 text-sm",
                      statusColors[service.current_status],
                    )}
                  >
                    <span>{SERVICE_STATUS_LABELS[service.current_status]}</span>
                    <Icon className="h-5 w-5" />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
