import { notFound } from "next/navigation";
import { verifyPublicSubscriber } from "@/lib/api-client";
import { PublicMessagePage } from "@/components/status/public-message-page";
import { resolveCustomDomainFromHeaders } from "@/lib/custom-domain";

interface Props {
  searchParams: Promise<{ token?: string }>;
}

export default async function CustomDomainVerifyPage({ searchParams }: Props) {
  const resolvedCustomDomain = await resolveCustomDomainFromHeaders();
  if (!resolvedCustomDomain) {
    notFound();
  }

  const { token } = await searchParams;
  let message = "Verification link is missing.";
  let title = "Verification failed";

  if (token) {
    try {
      const response = await verifyPublicSubscriber(resolvedCustomDomain.slug, token);
      message = response.message;
      title = "Subscription confirmed";
    } catch (error) {
      message = error instanceof Error ? error.message : "Verification failed.";
    }
  }

  return (
    <PublicMessagePage
      slug={resolvedCustomDomain.slug}
      title={title}
      message={message}
      resolvedCustomDomain={resolvedCustomDomain}
    />
  );
}
