"use client";

import { useState } from "react";
import type { ServiceUptime, UptimeDay } from "@/lib/types";

function getBarColor(day: UptimeDay): string {
  if (day.uptime_percentage === null) return "#D1D5DB"; // gray - no data
  if (day.uptime_percentage === 100) return "#10B981"; // green
  if (day.uptime_percentage >= 99) return "#34D399"; // light green
  if (day.uptime_percentage >= 95) return "#FBBF24"; // yellow
  if (day.uptime_percentage >= 90) return "#F97316"; // orange
  return "#EF4444"; // red
}

export function UptimeChart({ services }: { services: ServiceUptime[] }) {
  if (services.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">No services to display.</p>
    );
  }

  return (
    <div className="space-y-6">
      {services.map((service) => (
        <ServiceUptimeRow key={service.service_id} service={service} />
      ))}
    </div>
  );
}

function ServiceUptimeRow({ service }: { service: ServiceUptime }) {
  const [tooltip, setTooltip] = useState<{
    day: UptimeDay;
    x: number;
    y: number;
  } | null>(null);

  return (
    <div>
      <div className="mb-1 flex items-center justify-between">
        <span className="text-sm font-medium">{service.service_name}</span>
        <span className="text-sm text-muted-foreground">
          {service.overall_uptime !== null
            ? `${service.overall_uptime.toFixed(2)}% uptime`
            : "No data"}
        </span>
      </div>
      <div className="relative">
        <div className="flex gap-px">
          {service.days.map((day, i) => (
            <div
              key={i}
              className="h-8 flex-1 rounded-sm cursor-pointer transition-opacity hover:opacity-80"
              style={{ backgroundColor: getBarColor(day), minWidth: "2px" }}
              onMouseEnter={(e) => {
                const rect = e.currentTarget.getBoundingClientRect();
                setTooltip({
                  day,
                  x: rect.left + rect.width / 2,
                  y: rect.top,
                });
              }}
              onMouseLeave={() => setTooltip(null)}
            />
          ))}
        </div>
        {tooltip && (
          <div
            className="absolute z-10 -top-16 rounded-md border bg-popover px-3 py-2 text-xs shadow-md"
            style={{
              left: `${Math.max(0, Math.min(90, ((tooltip.x - (tooltip.x - 45)) / 400) * 100))}%`,
            }}
          >
            <div className="font-medium">
              {new Date(tooltip.day.date).toLocaleDateString(undefined, {
                month: "short",
                day: "numeric",
                year: "numeric",
              })}
            </div>
            <div>
              {tooltip.day.uptime_percentage !== null
                ? `${tooltip.day.uptime_percentage.toFixed(2)}%`
                : "No data"}
            </div>
            {tooltip.day.avg_response_time_ms !== null && (
              <div>{Math.round(tooltip.day.avg_response_time_ms)}ms avg</div>
            )}
          </div>
        )}
      </div>
      <div className="mt-1 flex justify-between text-xs text-muted-foreground">
        <span>90 days ago</span>
        <span>Today</span>
      </div>
    </div>
  );
}
