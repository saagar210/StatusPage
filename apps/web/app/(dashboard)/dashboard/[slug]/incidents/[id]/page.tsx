"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { toast } from "sonner";
import type { IncidentWithDetails, IncidentStatus } from "@/lib/types";
import { INCIDENT_STATUS_LABELS, INCIDENT_IMPACT_LABELS } from "@/lib/types";

const statusColors: Record<IncidentStatus, string> = {
  investigating: "bg-red-500",
  identified: "bg-orange-500",
  monitoring: "bg-yellow-500",
  resolved: "bg-green-500",
};

export default function IncidentDetailPage() {
  const params = useParams<{ slug: string; id: string }>();
  const { slug, id } = params;
  const [incident, setIncident] = useState<IncidentWithDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [updateStatus, setUpdateStatus] = useState<IncidentStatus>("monitoring");
  const [updateMessage, setUpdateMessage] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function fetchIncident() {
    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/incidents/${id}`,
      );
      if (res.ok) {
        const body = await res.json();
        setIncident(body.data);
      }
    } catch {
      toast.error("Failed to load incident");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchIncident();
  }, [slug, id]);

  async function handlePostUpdate(e: React.FormEvent) {
    e.preventDefault();
    if (!updateMessage.trim()) return;
    setSubmitting(true);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/incidents/${id}/updates`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            status: updateStatus,
            message: updateMessage.trim(),
          }),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to post update");
      }

      toast.success("Update posted");
      setUpdateMessage("");
      fetchIncident();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setSubmitting(false);
    }
  }

  if (loading) {
    return <div className="text-muted-foreground">Loading...</div>;
  }

  if (!incident) {
    return <div className="text-muted-foreground">Incident not found.</div>;
  }

  return (
    <div className="max-w-3xl space-y-6">
      <div>
        <h1 className="text-3xl font-bold">{incident.title}</h1>
        <div className="mt-2 flex items-center gap-3">
          <Badge variant="secondary">
            {INCIDENT_STATUS_LABELS[incident.status]}
          </Badge>
          <Badge variant="outline">
            {INCIDENT_IMPACT_LABELS[incident.impact]} impact
          </Badge>
          <span className="text-sm text-muted-foreground">
            Affecting:{" "}
            {incident.affected_services.map((s) => s.service_name).join(", ")}
          </span>
        </div>
      </div>

      {incident.status !== "resolved" && (
        <Card>
          <CardHeader>
            <CardTitle>Post Update</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handlePostUpdate} className="space-y-4">
              <div className="space-y-2">
                <Label>Status</Label>
                <Select
                  value={updateStatus}
                  onValueChange={(v) =>
                    setUpdateStatus(v as IncidentStatus)
                  }
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="investigating">Investigating</SelectItem>
                    <SelectItem value="identified">Identified</SelectItem>
                    <SelectItem value="monitoring">Monitoring</SelectItem>
                    <SelectItem value="resolved">Resolved</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label>Message</Label>
                <Textarea
                  value={updateMessage}
                  onChange={(e) => setUpdateMessage(e.target.value)}
                  placeholder="Provide an update on the situation..."
                  required
                  rows={3}
                />
              </div>
              <Button type="submit" disabled={submitting}>
                {submitting ? "Posting..." : "Post Update"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Timeline</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-6">
            {incident.updates.map((update, i) => (
              <div key={update.id}>
                {i > 0 && <Separator className="mb-6" />}
                <div className="flex gap-4">
                  <div className="flex flex-col items-center">
                    <div
                      className={`h-3 w-3 rounded-full ${statusColors[update.status]}`}
                    />
                    {i < incident.updates.length - 1 && (
                      <div className="w-px flex-1 bg-border" />
                    )}
                  </div>
                  <div className="flex-1 pb-4">
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-xs">
                        {INCIDENT_STATUS_LABELS[update.status]}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {new Date(update.created_at).toLocaleString()}
                      </span>
                    </div>
                    <p className="mt-2 text-sm">{update.message}</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
