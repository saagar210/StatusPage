"use client";

import { useEffect, useState, useCallback } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import type { Monitor, MonitorCheck } from "@/lib/types";

interface MonitorDetail {
  data: Monitor;
}

interface CheckHistory {
  data: MonitorCheck[];
  pagination: { page: number; per_page: number; total: number };
}

export default function MonitorDetailPage() {
  const params = useParams<{ slug: string; id: string }>();
  const router = useRouter();
  const [monitor, setMonitor] = useState<Monitor | null>(null);
  const [checks, setChecks] = useState<MonitorCheck[]>([]);
  const [pagination, setPagination] = useState({
    page: 1,
    per_page: 50,
    total: 0,
  });
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const [monitorRes, checksRes] = await Promise.all([
        fetch(`/api/proxy/organizations/${params.slug}/monitors/${params.id}`),
        fetch(
          `/api/proxy/organizations/${params.slug}/monitors/${params.id}/checks?per_page=50`
        ),
      ]);

      if (!monitorRes.ok) {
        router.push(`/dashboard/${params.slug}/monitors`);
        return;
      }

      const monitorData: MonitorDetail = await monitorRes.json();
      setMonitor(monitorData.data);

      if (checksRes.ok) {
        const checksData: CheckHistory = await checksRes.json();
        setChecks(checksData.data);
        setPagination(checksData.pagination);
      }
    } catch {
      // Ignore fetch errors
    } finally {
      setLoading(false);
    }
  }, [params.slug, params.id, router]);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, [fetchData]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-muted-foreground">Loading monitor...</div>
      </div>
    );
  }

  if (!monitor) return null;

  const config = monitor.config as Record<string, string | number | undefined>;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <Link
            href={`/dashboard/${params.slug}/monitors`}
            className="text-sm text-muted-foreground hover:underline"
          >
            &larr; Back to monitors
          </Link>
          <h1 className="mt-1 text-2xl font-bold">
            {monitor.monitor_type.toUpperCase()} Monitor
          </h1>
        </div>
        <span
          className={`inline-flex items-center rounded-full px-3 py-1 text-sm font-medium ${
            monitor.is_active
              ? monitor.consecutive_failures > 0
                ? "bg-red-100 text-red-800"
                : "bg-green-100 text-green-800"
              : "bg-gray-100 text-gray-800"
          }`}
        >
          {!monitor.is_active
            ? "Paused"
            : monitor.consecutive_failures > 0
              ? "Down"
              : "Up"}
        </span>
      </div>

      {/* Monitor Config Summary */}
      <div className="rounded-lg border p-4">
        <h2 className="mb-3 text-sm font-semibold text-muted-foreground">
          Configuration
        </h2>
        <dl className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <dt className="text-muted-foreground">Type</dt>
            <dd className="font-medium">{monitor.monitor_type}</dd>
          </div>
          {config.url && (
            <div>
              <dt className="text-muted-foreground">URL</dt>
              <dd className="font-medium truncate">
                {config.url as string}
              </dd>
            </div>
          )}
          {config.host && (
            <div>
              <dt className="text-muted-foreground">Host</dt>
              <dd className="font-medium">
                {config.host as string}
                {config.port ? `:${config.port}` : ""}
              </dd>
            </div>
          )}
          {config.hostname && (
            <div>
              <dt className="text-muted-foreground">Hostname</dt>
              <dd className="font-medium">{config.hostname as string}</dd>
            </div>
          )}
          <div>
            <dt className="text-muted-foreground">Interval</dt>
            <dd className="font-medium">{monitor.interval_seconds}s</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Timeout</dt>
            <dd className="font-medium">{monitor.timeout_ms}ms</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Failure Threshold</dt>
            <dd className="font-medium">{monitor.failure_threshold}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Consecutive Failures</dt>
            <dd className="font-medium">{monitor.consecutive_failures}</dd>
          </div>
          {monitor.last_checked_at && (
            <div>
              <dt className="text-muted-foreground">Last Checked</dt>
              <dd className="font-medium">
                {new Date(monitor.last_checked_at).toLocaleString()}
              </dd>
            </div>
          )}
          {monitor.last_response_time_ms != null && (
            <div>
              <dt className="text-muted-foreground">Last Response Time</dt>
              <dd className="font-medium">{monitor.last_response_time_ms}ms</dd>
            </div>
          )}
        </dl>
      </div>

      {/* Check History */}
      <div>
        <h2 className="mb-3 text-sm font-semibold text-muted-foreground">
          Recent Checks ({pagination.total} total)
        </h2>
        {checks.length === 0 ? (
          <div className="rounded-lg border p-6 text-center text-muted-foreground">
            No checks recorded yet. Checks will appear once the monitor engine
            runs.
          </div>
        ) : (
          <div className="rounded-lg border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="px-4 py-2 text-left font-medium">Time</th>
                  <th className="px-4 py-2 text-left font-medium">Status</th>
                  <th className="px-4 py-2 text-left font-medium">
                    Response Time
                  </th>
                  <th className="px-4 py-2 text-left font-medium">
                    Status Code
                  </th>
                  <th className="px-4 py-2 text-left font-medium">Error</th>
                </tr>
              </thead>
              <tbody>
                {checks.map((check) => (
                  <tr key={check.id} className="border-b last:border-0">
                    <td className="px-4 py-2 text-muted-foreground">
                      {new Date(check.checked_at).toLocaleString()}
                    </td>
                    <td className="px-4 py-2">
                      <span
                        className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                          check.status === "success"
                            ? "bg-green-100 text-green-800"
                            : check.status === "timeout"
                              ? "bg-yellow-100 text-yellow-800"
                              : "bg-red-100 text-red-800"
                        }`}
                      >
                        {check.status}
                      </span>
                    </td>
                    <td className="px-4 py-2">
                      {check.response_time_ms != null
                        ? `${check.response_time_ms}ms`
                        : "—"}
                    </td>
                    <td className="px-4 py-2">
                      {check.status_code ?? "—"}
                    </td>
                    <td className="px-4 py-2 text-red-600 truncate max-w-xs">
                      {check.error_message ?? "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
