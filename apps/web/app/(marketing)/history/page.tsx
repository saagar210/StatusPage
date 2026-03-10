import { notFound } from "next/navigation";
import type { Metadata } from "next";
import { PublicHistoryPageContent } from "@/components/status/public-history-page";
import { resolveCustomDomainFromHeaders } from "@/lib/custom-domain";

interface Props {
  searchParams: Promise<{ page?: string }>;
}

export async function generateMetadata(): Promise<Metadata> {
  const resolvedCustomDomain = await resolveCustomDomainFromHeaders();

  if (!resolvedCustomDomain) {
    return { title: "Incident History" };
  }

  return {
    title: `Incident History - ${resolvedCustomDomain.organization.name} Status`,
  };
}

export default async function CustomDomainHistoryPage({ searchParams }: Props) {
  const resolvedCustomDomain = await resolveCustomDomainFromHeaders();
  if (!resolvedCustomDomain) {
    notFound();
  }

  const { page: pageParam } = await searchParams;
  const page = parseInt(pageParam ?? "1", 10) || 1;

  return (
    <div className="mx-auto max-w-3xl px-4 py-8">
      <PublicHistoryPageContent
        slug={resolvedCustomDomain.slug}
        page={page}
        resolvedCustomDomain={resolvedCustomDomain}
      />
    </div>
  );
}
