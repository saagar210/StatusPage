import Link from "next/link";
import { buildPublicBasePath, buildPublicHref } from "@/lib/custom-domain";
import type { ResolvedCustomDomain } from "@/lib/types";

export function PublicMessagePage({
  slug,
  title,
  message,
  resolvedCustomDomain = null,
}: {
  slug: string;
  title: string;
  message: string;
  resolvedCustomDomain?: ResolvedCustomDomain | null;
}) {
  const basePath = buildPublicBasePath(slug, resolvedCustomDomain);

  return (
    <div className="mx-auto max-w-xl px-4 py-12">
      <div className="rounded-lg border p-6">
        <h1 className="text-2xl font-bold">{title}</h1>
        <p className="mt-3 text-muted-foreground">{message}</p>
        <Link
          href={buildPublicHref(basePath)}
          className="mt-6 inline-block text-sm underline"
        >
          Back to status page
        </Link>
      </div>
    </div>
  );
}
