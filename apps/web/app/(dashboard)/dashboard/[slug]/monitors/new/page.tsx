"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toast } from "sonner";
import type { Service, MonitorType } from "@/lib/types";

export default function NewMonitorPage() {
  const params = useParams<{ slug: string }>();
  const router = useRouter();
  const slug = params.slug;

  const [services, setServices] = useState<Service[]>([]);
  const [serviceId, setServiceId] = useState("");
  const [monitorType, setMonitorType] = useState<MonitorType>("http");
  const [url, setUrl] = useState("");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("443");
  const [hostname, setHostname] = useState("");
  const [interval, setInterval_] = useState("60");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    async function loadServices() {
      const res = await fetch(`/api/proxy/api/organizations/${slug}/services`);
      if (res.ok) {
        const body = await res.json();
        setServices(body.data);
      }
    }
    loadServices();
  }, [slug]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!serviceId) {
      toast.error("Select a service");
      return;
    }
    setLoading(true);

    let config: Record<string, unknown>;
    switch (monitorType) {
      case "http":
        config = { type: "http", url, method: "GET", expected_status: 200, headers: {} };
        break;
      case "tcp":
        config = { type: "tcp", host, port: parseInt(port) };
        break;
      case "dns":
        config = { type: "dns", hostname };
        break;
      case "ping":
        config = { type: "ping", host };
        break;
    }

    try {
      const res = await fetch(`/api/proxy/api/organizations/${slug}/monitors`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          service_id: serviceId,
          monitor_type: monitorType,
          config,
          interval_seconds: parseInt(interval),
        }),
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to create monitor");
      }

      toast.success("Monitor created");
      router.push(`/dashboard/${slug}/monitors`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-2xl space-y-6">
      <h1 className="text-3xl font-bold">Add Monitor</h1>

      <Card>
        <CardHeader>
          <CardTitle>Monitor Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label>Service</Label>
              <Select value={serviceId} onValueChange={setServiceId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select a service" />
                </SelectTrigger>
                <SelectContent>
                  {services.map((s) => (
                    <SelectItem key={s.id} value={s.id}>
                      {s.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>Type</Label>
              <Select
                value={monitorType}
                onValueChange={(v) => setMonitorType(v as MonitorType)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="http">HTTP</SelectItem>
                  <SelectItem value="tcp">TCP</SelectItem>
                  <SelectItem value="dns">DNS</SelectItem>
                  <SelectItem value="ping">Ping</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {monitorType === "http" && (
              <div className="space-y-2">
                <Label>URL</Label>
                <Input
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="https://api.example.com/health"
                  required
                />
              </div>
            )}

            {monitorType === "tcp" && (
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Host</Label>
                  <Input
                    value={host}
                    onChange={(e) => setHost(e.target.value)}
                    placeholder="example.com"
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label>Port</Label>
                  <Input
                    type="number"
                    value={port}
                    onChange={(e) => setPort(e.target.value)}
                    required
                  />
                </div>
              </div>
            )}

            {monitorType === "dns" && (
              <div className="space-y-2">
                <Label>Hostname</Label>
                <Input
                  value={hostname}
                  onChange={(e) => setHostname(e.target.value)}
                  placeholder="example.com"
                  required
                />
              </div>
            )}

            {monitorType === "ping" && (
              <div className="space-y-2">
                <Label>Host</Label>
                <Input
                  value={host}
                  onChange={(e) => setHost(e.target.value)}
                  placeholder="example.com"
                  required
                />
              </div>
            )}

            <div className="space-y-2">
              <Label>Check Interval</Label>
              <Select value={interval} onValueChange={setInterval_}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="30">Every 30 seconds</SelectItem>
                  <SelectItem value="60">Every 60 seconds</SelectItem>
                  <SelectItem value="120">Every 2 minutes</SelectItem>
                  <SelectItem value="300">Every 5 minutes</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? "Creating..." : "Create Monitor"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
