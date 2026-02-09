"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { StatusBadge } from "@/components/dashboard/status-badge";
import { ServiceForm } from "@/components/dashboard/service-form";
import { Plus, Trash2, Pencil } from "lucide-react";
import { toast } from "sonner";
import type { Service } from "@/lib/types";

export default function ServicesPage() {
  const params = useParams<{ slug: string }>();
  const slug = params.slug;
  const [services, setServices] = useState<Service[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingService, setEditingService] = useState<Service | null>(null);

  async function fetchServices() {
    try {
      const res = await fetch(`/api/proxy/api/organizations/${slug}/services`);
      if (res.ok) {
        const body = await res.json();
        setServices(body.data);
      }
    } catch {
      toast.error("Failed to load services");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchServices();
  }, [slug]);

  async function handleDelete(serviceId: string) {
    if (!confirm("Are you sure you want to delete this service?")) return;
    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/services/${serviceId}`,
        { method: "DELETE" },
      );
      if (res.ok) {
        toast.success("Service deleted");
        fetchServices();
      } else {
        toast.error("Failed to delete service");
      }
    } catch {
      toast.error("Failed to delete service");
    }
  }

  function handleCreated() {
    setDialogOpen(false);
    setEditingService(null);
    fetchServices();
  }

  if (loading) {
    return <div className="text-muted-foreground">Loading services...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Services</h1>
        <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogTrigger asChild>
            <Button>
              <Plus className="mr-2 h-4 w-4" />
              Add Service
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {editingService ? "Edit Service" : "Add Service"}
              </DialogTitle>
            </DialogHeader>
            <ServiceForm
              slug={slug}
              service={editingService}
              onSuccess={handleCreated}
            />
          </DialogContent>
        </Dialog>
      </div>

      {services.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground">No services yet.</p>
            <p className="text-sm text-muted-foreground">
              Add your first service to get started.
            </p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>All Services</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {services.map((service) => (
                <div
                  key={service.id}
                  className="flex items-center justify-between rounded-lg border p-4"
                >
                  <div className="flex-1">
                    <div className="flex items-center gap-3">
                      <span className="font-medium">{service.name}</span>
                      {service.group_name && (
                        <span className="text-xs text-muted-foreground">
                          {service.group_name}
                        </span>
                      )}
                      {!service.is_visible && (
                        <span className="text-xs text-muted-foreground">
                          (hidden)
                        </span>
                      )}
                    </div>
                    {service.description && (
                      <p className="mt-1 text-sm text-muted-foreground">
                        {service.description}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-3">
                    <StatusBadge status={service.current_status} />
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        setEditingService(service);
                        setDialogOpen(true);
                      }}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDelete(service.id)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
