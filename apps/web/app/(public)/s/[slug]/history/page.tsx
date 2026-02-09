import { getPublicStatus, getPublicIncidents } from "@/lib/api-client";
import { notFound } from "next/navigation";
import type { Metadata } from "next";
import Link from "next/link";
import {
  INCIDENT_STATUS_LABELS,
  INCIDENT_IMPACT_LABELS,
  INCIDENT_IMPACT_COLORS,
} from "@/lib/types";

interface Props {
  params: Promise<{ slug: string }>;
  searchParams: Promise<{ page?: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  try {
    const status = await getPublicStatus(slug);
    return {
      title: `Incident History - ${status.organization.name} Status`,
    };
  } catch {
    return { title: "Incident History" };
  }
}

export default async function IncidentHistoryPage({
  params,
  searchParams,
}: Props) {
  const { slug } = await params;
  const { page: pageParam } = await searchParams;
  const page = parseInt(pageParam ?? "1", 10) || 1;

  let status;
  try {
    status = await getPublicStatus(slug);
  } catch {
    notFound();
  }

  const { data: incidents, pagination } = await getPublicIncidents(
    slug,
    page,
    20
  );

  const totalPages = Math.ceil(pagination.total / pagination.per_page);

  // Group incidents by date
  const grouped = new Map<string, typeof incidents>();
  for (const incident of incidents) {
    const date = new Date(incident.started_at).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric",
    });
    if (!grouped.has(date)) grouped.set(date, []);
    grouped.get(date)!.push(incident);
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Incident History</h1>
        <Link
          href={`/s/${slug}`}
          className="text-sm text-muted-foreground hover:underline"
        >
          Back to status
        </Link>
      </div>

      {incidents.length === 0 ? (
        <div className="rounded-lg border p-8 text-center text-muted-foreground">
          No incidents found.
        </div>
      ) : (
        <div className="space-y-6">
          {Array.from(grouped.entries()).map(([date, dayIncidents]) => (
            <div key={date}>
              <h2 className="mb-3 text-sm font-semibold text-muted-foreground">
                {date}
              </h2>
              <div className="space-y-3">
                {dayIncidents.map((incident) => (
                  <div key={incident.id} className="rounded-lg border p-4">
                    <div className="flex items-start justify-between">
                      <div>
                        <h3 className="font-medium">{incident.title}</h3>
                        <div className="mt-1 flex items-center gap-2 text-sm">
                          <span
                            className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                              INCIDENT_IMPACT_COLORS[incident.impact] ??
                              "bg-gray-100 text-gray-800"
                            }`}
                          >
                            {INCIDENT_IMPACT_LABELS[incident.impact] ??
                              incident.impact}
                          </span>
                          <span className="text-muted-foreground">
                            {INCIDENT_STATUS_LABELS[incident.status] ??
                              incident.status}
                          </span>
                        </div>
                      </div>
                      {incident.resolved_at && (
                        <span className="text-xs text-muted-foreground">
                          Duration:{" "}
                          {formatDuration(
                            incident.started_at,
                            incident.resolved_at
                          )}
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-4">
          {page > 1 && (
            <Link
              href={`/s/${slug}/history?page=${page - 1}`}
              className="text-sm hover:underline"
            >
              Previous
            </Link>
          )}
          <span className="text-sm text-muted-foreground">
            Page {page} of {totalPages}
          </span>
          {page < totalPages && (
            <Link
              href={`/s/${slug}/history?page=${page + 1}`}
              className="text-sm hover:underline"
            >
              Next
            </Link>
          )}
        </div>
      )}
    </div>
  );
}

function formatDuration(start: string, end: string): string {
  const ms = new Date(end).getTime() - new Date(start).getTime();
  const minutes = Math.floor(ms / 60000);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ${minutes % 60}m`;
  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}
