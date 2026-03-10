import type { Metadata } from "next";
import { getPublicStatus } from "@/lib/api-client";
import { PublicHistoryPageContent } from "@/components/status/public-history-page";

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

  return <PublicHistoryPageContent slug={slug} page={page} />;
}
