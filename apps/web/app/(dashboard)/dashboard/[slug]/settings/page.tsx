"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { toast } from "sonner";
import { formatPlanMonitorLimit } from "@/lib/types";
import type { Organization } from "@/lib/types";

export default function SettingsPage() {
  const params = useParams<{ slug: string }>();
  const slug = params.slug;
  const [org, setOrg] = useState<Organization | null>(null);
  const [name, setName] = useState("");
  const [brandColor, setBrandColor] = useState("#3B82F6");
  const [loading, setLoading] = useState(false);
  const [monitorCount, setMonitorCount] = useState<number | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    async function loadOrg() {
      setLoadError(null);
      try {
        const [orgRes, monitorRes] = await Promise.all([
          fetch(`/api/proxy/api/organizations/${slug}`),
          fetch(`/api/proxy/api/organizations/${slug}/monitors`),
        ]);

        if (!orgRes.ok) {
          throw new Error("Failed to load organization settings");
        }

        const orgBody = await orgRes.json();
        setOrg(orgBody.data);
        setName(orgBody.data.name);
        setBrandColor(orgBody.data.brand_color);

        if (monitorRes.ok) {
          const monitorBody = await monitorRes.json();
          setMonitorCount(monitorBody.data.length);
        } else {
          setMonitorCount(null);
          toast.error("Failed to load monitor usage");
        }
      } catch {
        setLoadError("Failed to load settings");
        toast.error("Failed to load settings");
      }
    }

    loadOrg();
  }, [slug]);

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);

    try {
      const res = await fetch(`/api/proxy/api/organizations/${slug}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: name.trim(), brand_color: brandColor }),
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to update");
      }

      toast.success("Settings saved");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  if (!org) {
    return (
      <div className="text-muted-foreground">
        {loadError ?? "Loading..."}
      </div>
    );
  }

  return (
    <div className="max-w-2xl space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      <Card>
        <CardHeader>
          <CardTitle>Plan & Usage</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">Current plan</span>
            <Badge variant="secondary" className="uppercase">
              {org.plan}
            </Badge>
          </div>
          <p className="text-sm text-muted-foreground">
            Monitors: {monitorCount ?? "..."} / {formatPlanMonitorLimit(org.plan)}
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Organization</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSave} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="slug">Slug</Label>
              <Input id="slug" value={org.slug} disabled />
              <p className="text-xs text-muted-foreground">
                Public page: /s/{org.slug}
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="color">Brand Color</Label>
              <div className="flex gap-2">
                <Input
                  type="color"
                  id="color"
                  value={brandColor}
                  onChange={(e) => setBrandColor(e.target.value)}
                  className="h-10 w-14 p-1"
                />
                <Input
                  value={brandColor}
                  onChange={(e) => setBrandColor(e.target.value)}
                  placeholder="#3B82F6"
                />
              </div>
            </div>
            <Button type="submit" disabled={loading}>
              {loading ? "Saving..." : "Save Changes"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
