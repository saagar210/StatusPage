import { cn } from "@/lib/utils";
import type { ServiceStatus } from "@/lib/types";
import { CheckCircle, AlertTriangle, XCircle, Wrench } from "lucide-react";

const bannerConfig: Record<
  ServiceStatus,
  { bg: string; text: string; icon: React.ElementType; label: string }
> = {
  operational: {
    bg: "bg-green-500",
    text: "text-white",
    icon: CheckCircle,
    label: "All Systems Operational",
  },
  degraded_performance: {
    bg: "bg-yellow-500",
    text: "text-white",
    icon: AlertTriangle,
    label: "Degraded System Performance",
  },
  partial_outage: {
    bg: "bg-orange-500",
    text: "text-white",
    icon: AlertTriangle,
    label: "Partial System Outage",
  },
  major_outage: {
    bg: "bg-red-500",
    text: "text-white",
    icon: XCircle,
    label: "Major System Outage",
  },
  under_maintenance: {
    bg: "bg-blue-500",
    text: "text-white",
    icon: Wrench,
    label: "Scheduled Maintenance In Progress",
  },
};

export function StatusBanner({
  overallStatus,
}: {
  overallStatus: ServiceStatus;
}) {
  const config = bannerConfig[overallStatus];
  const Icon = config.icon;

  return (
    <div
      className={cn(
        "flex items-center justify-center gap-3 rounded-lg p-4",
        config.bg,
        config.text,
      )}
    >
      <Icon className="h-6 w-6" />
      <span className="text-lg font-semibold">{config.label}</span>
    </div>
  );
}
