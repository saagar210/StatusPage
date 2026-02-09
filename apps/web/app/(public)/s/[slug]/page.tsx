import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { getPublicStatus, getPublicUptime } from "@/lib/api-client";
import { StatusBanner } from "@/components/status/status-banner";
import { ServiceList } from "@/components/status/service-list";
import { UptimeChart } from "@/components/status/uptime-chart";
import { ActiveIncidents } from "@/components/status/active-incidents";
import type { PublicStatusResponse, UptimeResponse } from "@/lib/types";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  try {
    const status = await getPublicStatus(slug);
    return {
      title: `${status.organization.name} Status`,
      description: `Current status and uptime for ${status.organization.name}`,
    };
  } catch {
    return { title: "Status Page" };
  }
}

export default async function PublicStatusPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;

  let status: PublicStatusResponse;
  let uptime: UptimeResponse;

  try {
    [status, uptime] = await Promise.all([
      getPublicStatus(slug),
      getPublicUptime(slug),
    ]);
  } catch {
    notFound();
  }

  return (
    <div className="mx-auto max-w-3xl px-4 py-8">
      <header className="mb-8 text-center">
        <h1 className="text-2xl font-bold">{status.organization.name}</h1>
      </header>

      <StatusBanner overallStatus={status.overall_status} />

      {status.active_incidents.length > 0 && (
        <div className="mt-8">
          <ActiveIncidents incidents={status.active_incidents} />
        </div>
      )}

      <div className="mt-8">
        <ServiceList services={status.services} />
      </div>

      <div className="mt-8">
        <h2 className="mb-4 text-lg font-semibold">Uptime (90 days)</h2>
        <UptimeChart services={uptime.services} />
      </div>
    </div>
  );
}
