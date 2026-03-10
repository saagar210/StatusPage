import { notFound } from "next/navigation";
import { getPublicStatus, getPublicUptime } from "@/lib/api-client";
import { buildPublicBasePath, buildPublicHref } from "@/lib/custom-domain";
import type { ResolvedCustomDomain, PublicStatusResponse, UptimeResponse } from "@/lib/types";
import { StatusBanner } from "@/components/status/status-banner";
import { ServiceList } from "@/components/status/service-list";
import { UptimeChart } from "@/components/status/uptime-chart";
import { ActiveIncidents } from "@/components/status/active-incidents";
import { SubscribeForm } from "@/components/status/subscribe-form";
import Link from "next/link";

export async function PublicStatusPageContent({
  slug,
  resolvedCustomDomain = null,
}: {
  slug: string;
  resolvedCustomDomain?: ResolvedCustomDomain | null;
}) {
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

  const basePath = buildPublicBasePath(slug, resolvedCustomDomain);

  return (
    <div className="mx-auto max-w-3xl px-4 py-8">
      <header className="mb-8 text-center">
        <h1 className="text-2xl font-bold">{status.organization.name}</h1>
        <div className="mt-3">
          <Link
            href={buildPublicHref(basePath, "/history")}
            className="text-sm text-muted-foreground hover:underline"
          >
            Incident history
          </Link>
        </div>
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

      <div className="mt-8">
        <SubscribeForm slug={slug} />
      </div>
    </div>
  );
}
