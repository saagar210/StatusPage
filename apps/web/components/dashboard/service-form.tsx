"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toast } from "sonner";
import type { Service, ServiceStatus } from "@/lib/types";

interface ServiceFormProps {
  slug: string;
  service?: Service | null;
  onSuccess: () => void;
}

export function ServiceForm({ slug, service, onSuccess }: ServiceFormProps) {
  const [name, setName] = useState(service?.name || "");
  const [description, setDescription] = useState(service?.description || "");
  const [groupName, setGroupName] = useState(service?.group_name || "");
  const [status, setStatus] = useState<ServiceStatus>(
    service?.current_status || "operational",
  );
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);

    const body = {
      name: name.trim(),
      description: description.trim() || null,
      group_name: groupName.trim() || null,
      ...(service ? { current_status: status } : {}),
    };

    try {
      const url = service
        ? `/api/proxy/api/organizations/${slug}/services/${service.id}`
        : `/api/proxy/api/organizations/${slug}/services`;

      const res = await fetch(url, {
        method: service ? "PATCH" : "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed");
      }

      toast.success(service ? "Service updated" : "Service created");
      onSuccess();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor="name">Name</Label>
        <Input
          id="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="API Server"
          required
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="description">Description (optional)</Label>
        <Textarea
          id="description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Main API endpoint"
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="group">Group (optional)</Label>
        <Input
          id="group"
          value={groupName}
          onChange={(e) => setGroupName(e.target.value)}
          placeholder="Core Infrastructure"
        />
      </div>
      {service && (
        <div className="space-y-2">
          <Label>Status</Label>
          <Select
            value={status}
            onValueChange={(v) => setStatus(v as ServiceStatus)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="operational">Operational</SelectItem>
              <SelectItem value="degraded_performance">
                Degraded Performance
              </SelectItem>
              <SelectItem value="partial_outage">Partial Outage</SelectItem>
              <SelectItem value="major_outage">Major Outage</SelectItem>
              <SelectItem value="under_maintenance">
                Under Maintenance
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      )}
      <Button type="submit" className="w-full" disabled={loading}>
        {loading ? "Saving..." : service ? "Update Service" : "Create Service"}
      </Button>
    </form>
  );
}
