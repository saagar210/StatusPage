import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { ServiceStatus } from "@/lib/types";
import { SERVICE_STATUS_LABELS } from "@/lib/types";

const statusStyles: Record<ServiceStatus, string> = {
  operational: "bg-green-500/10 text-green-700 border-green-200",
  degraded_performance: "bg-yellow-500/10 text-yellow-700 border-yellow-200",
  partial_outage: "bg-orange-500/10 text-orange-700 border-orange-200",
  major_outage: "bg-red-500/10 text-red-700 border-red-200",
  under_maintenance: "bg-blue-500/10 text-blue-700 border-blue-200",
};

const dotColors: Record<ServiceStatus, string> = {
  operational: "bg-green-500",
  degraded_performance: "bg-yellow-500",
  partial_outage: "bg-orange-500",
  major_outage: "bg-red-500",
  under_maintenance: "bg-blue-500",
};

export function StatusBadge({ status }: { status: ServiceStatus }) {
  return (
    <Badge variant="outline" className={cn("gap-1.5", statusStyles[status])}>
      <span className={cn("h-2 w-2 rounded-full", dotColors[status])} />
      {SERVICE_STATUS_LABELS[status]}
    </Badge>
  );
}
