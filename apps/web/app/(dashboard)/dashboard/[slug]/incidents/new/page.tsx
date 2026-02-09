"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
import type { Service, IncidentImpact } from "@/lib/types";

export default function NewIncidentPage() {
  const params = useParams<{ slug: string }>();
  const router = useRouter();
  const slug = params.slug;

  const [services, setServices] = useState<Service[]>([]);
  const [title, setTitle] = useState("");
  const [message, setMessage] = useState("");
  const [impact, setImpact] = useState<IncidentImpact>("minor");
  const [selectedServices, setSelectedServices] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    async function loadServices() {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/services`,
      );
      if (res.ok) {
        const body = await res.json();
        setServices(body.data);
      }
    }
    loadServices();
  }, [slug]);

  function toggleService(id: string) {
    setSelectedServices((prev) =>
      prev.includes(id) ? prev.filter((s) => s !== id) : [...prev, id],
    );
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (selectedServices.length === 0) {
      toast.error("Select at least one affected service");
      return;
    }
    setLoading(true);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/incidents`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            title: title.trim(),
            impact,
            message: message.trim(),
            affected_service_ids: selectedServices,
          }),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to create incident");
      }

      const { data } = await res.json();
      toast.success("Incident created");
      router.push(`/dashboard/${slug}/incidents/${data.id}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-2xl space-y-6">
      <h1 className="text-3xl font-bold">Create Incident</h1>

      <Card>
        <CardHeader>
          <CardTitle>Incident Details</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="title">Title</Label>
              <Input
                id="title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="Elevated error rates on API"
                required
              />
            </div>

            <div className="space-y-2">
              <Label>Impact</Label>
              <Select
                value={impact}
                onValueChange={(v) => setImpact(v as IncidentImpact)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">None</SelectItem>
                  <SelectItem value="minor">Minor</SelectItem>
                  <SelectItem value="major">Major</SelectItem>
                  <SelectItem value="critical">Critical</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>Affected Services</Label>
              <div className="space-y-2 rounded-md border p-3">
                {services.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    No services found. Create services first.
                  </p>
                ) : (
                  services.map((service) => (
                    <label
                      key={service.id}
                      className="flex items-center gap-2 cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={selectedServices.includes(service.id)}
                        onChange={() => toggleService(service.id)}
                        className="rounded"
                      />
                      <span className="text-sm">{service.name}</span>
                    </label>
                  ))
                )}
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="message">Initial Update Message</Label>
              <Textarea
                id="message"
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder="We are investigating elevated error rates..."
                required
                rows={4}
              />
            </div>

            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? "Creating..." : "Create Incident"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
