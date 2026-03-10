import { notFound } from "next/navigation";
import { unsubscribePublicSubscriber } from "@/lib/api-client";
import { PublicMessagePage } from "@/components/status/public-message-page";
import { resolveCustomDomainFromHeaders } from "@/lib/custom-domain";

interface Props {
  searchParams: Promise<{ token?: string }>;
}

export default async function CustomDomainUnsubscribePage({
  searchParams,
}: Props) {
  const resolvedCustomDomain = await resolveCustomDomainFromHeaders();
  if (!resolvedCustomDomain) {
    notFound();
  }

  const { token } = await searchParams;
  let message = "Unsubscribe link is missing.";
  let title = "Unable to unsubscribe";

  if (token) {
    try {
      const response = await unsubscribePublicSubscriber(
        resolvedCustomDomain.slug,
        token,
      );
      message = response.message;
      title = "Unsubscribed";
    } catch (error) {
      message = error instanceof Error ? error.message : "Unsubscribe failed.";
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
