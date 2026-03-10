import type { Metadata } from "next";
import { getPublicStatus } from "@/lib/api-client";
import { PublicStatusPageContent } from "@/components/status/public-status-page";

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
  return <PublicStatusPageContent slug={slug} />;
}
