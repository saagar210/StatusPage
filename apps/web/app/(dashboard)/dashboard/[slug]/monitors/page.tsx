"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import type { Monitor } from "@/lib/types";

export default function MonitorsPage() {
  const params = useParams<{ slug: string }>();
  const slug = params.slug;
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [loading, setLoading] = useState(true);

  async function fetchMonitors() {
    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/monitors`,
      );
      if (res.ok) {
        const body = await res.json();
        setMonitors(body.data);
      }
    } catch {
      toast.error("Failed to load monitors");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchMonitors();
  }, [slug]);

  async function handleDelete(monitorId: string) {
    if (!confirm("Are you sure?")) return;
    const res = await fetch(
      `/api/proxy/api/organizations/${slug}/monitors/${monitorId}`,
      { method: "DELETE" },
    );
    if (res.ok) {
      toast.success("Monitor deleted");
      fetchMonitors();
    }
  }

  async function handleToggle(monitor: Monitor) {
    const res = await fetch(
      `/api/proxy/api/organizations/${slug}/monitors/${monitor.id}`,
      {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ is_active: !monitor.is_active }),
      },
    );
    if (res.ok) {
      toast.success(
        monitor.is_active ? "Monitor paused" : "Monitor resumed",
      );
      fetchMonitors();
    }
  }

  if (loading) {
    return <div className="text-muted-foreground">Loading monitors...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Monitors</h1>
        <Button asChild>
          <Link href={`/dashboard/${slug}/monitors/new`}>
            <Plus className="mr-2 h-4 w-4" />
            Add Monitor
          </Link>
        </Button>
      </div>

      {monitors.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">No monitors configured.</p>
            <p className="text-sm text-muted-foreground">
              Add a monitor to start checking your services automatically.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {monitors.map((monitor) => (
            <Card key={monitor.id}>
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <CardTitle className="text-base">
                      {monitor.monitor_type.toUpperCase()}
                    </CardTitle>
                    <Badge variant={monitor.is_active ? "default" : "secondary"}>
                      {monitor.is_active ? "Active" : "Paused"}
                    </Badge>
                    {monitor.last_checked_at && (
                      <Badge
                        variant="outline"
                        className={
                          monitor.consecutive_failures > 0
                            ? "text-red-600"
                            : "text-green-600"
                        }
                      >
                        {monitor.consecutive_failures > 0 ? "Down" : "Up"}
                      </Badge>
                    )}
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleToggle(monitor)}
                    >
                      {monitor.is_active ? "Pause" : "Resume"}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDelete(monitor.id)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="flex gap-6 text-sm text-muted-foreground">
                  <span>Every {monitor.interval_seconds}s</span>
                  <span>Timeout {monitor.timeout_ms}ms</span>
                  <span>Threshold {monitor.failure_threshold} failures</span>
                  {monitor.last_response_time_ms !== null && (
                    <span>Last: {monitor.last_response_time_ms}ms</span>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
