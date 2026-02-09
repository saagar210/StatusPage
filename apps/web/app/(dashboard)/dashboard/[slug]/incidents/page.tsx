"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import type { Incident } from "@/lib/types";
import { INCIDENT_STATUS_LABELS, INCIDENT_IMPACT_LABELS } from "@/lib/types";

const impactColors: Record<string, string> = {
  none: "bg-gray-100 text-gray-700",
  minor: "bg-yellow-100 text-yellow-700",
  major: "bg-orange-100 text-orange-700",
  critical: "bg-red-100 text-red-700",
};

export default function IncidentsPage() {
  const params = useParams<{ slug: string }>();
  const slug = params.slug;
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetch_() {
      try {
        const res = await fetch(
          `/api/proxy/api/organizations/${slug}/incidents?per_page=50`,
        );
        if (res.ok) {
          const body = await res.json();
          setIncidents(body.data);
        }
      } catch {
        toast.error("Failed to load incidents");
      } finally {
        setLoading(false);
      }
    }
    fetch_();
  }, [slug]);

  const active = incidents.filter((i) => i.status !== "resolved");
  const resolved = incidents.filter((i) => i.status === "resolved");

  if (loading) {
    return <div className="text-muted-foreground">Loading incidents...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Incidents</h1>
        <Button asChild>
          <Link href={`/dashboard/${slug}/incidents/new`}>
            <Plus className="mr-2 h-4 w-4" />
            Create Incident
          </Link>
        </Button>
      </div>

      <Tabs defaultValue="active">
        <TabsList>
          <TabsTrigger value="active">Active ({active.length})</TabsTrigger>
          <TabsTrigger value="resolved">
            Resolved ({resolved.length})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="active">
          <IncidentList incidents={active} slug={slug} />
        </TabsContent>
        <TabsContent value="resolved">
          <IncidentList incidents={resolved} slug={slug} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function IncidentList({
  incidents,
  slug,
}: {
  incidents: Incident[];
  slug: string;
}) {
  if (incidents.length === 0) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          No incidents.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-3">
      {incidents.map((incident) => (
        <Link
          key={incident.id}
          href={`/dashboard/${slug}/incidents/${incident.id}`}
        >
          <Card className="cursor-pointer transition-colors hover:bg-accent">
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <CardTitle className="text-base">{incident.title}</CardTitle>
                <div className="flex gap-2">
                  <Badge
                    variant="outline"
                    className={impactColors[incident.impact]}
                  >
                    {INCIDENT_IMPACT_LABELS[incident.impact]}
                  </Badge>
                  <Badge variant="secondary">
                    {INCIDENT_STATUS_LABELS[incident.status]}
                  </Badge>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">
                Started{" "}
                {new Date(incident.started_at).toLocaleDateString(undefined, {
                  month: "short",
                  day: "numeric",
                  hour: "2-digit",
                  minute: "2-digit",
                })}
              </p>
            </CardContent>
          </Card>
        </Link>
      ))}
    </div>
  );
}
