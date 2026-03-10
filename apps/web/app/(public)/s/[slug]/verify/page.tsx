import { verifyPublicSubscriber } from "@/lib/api-client";
import { PublicMessagePage } from "@/components/status/public-message-page";

interface Props {
  params: Promise<{ slug: string }>;
  searchParams: Promise<{ token?: string }>;
}

export default async function VerifySubscriberPage({ params, searchParams }: Props) {
  const { slug } = await params;
  const { token } = await searchParams;

  let message = "Verification link is missing.";
  let title = "Verification failed";

  if (token) {
    try {
      const response = await verifyPublicSubscriber(slug, token);
      message = response.message;
      title = "Subscription confirmed";
    } catch (error) {
      message = error instanceof Error ? error.message : "Verification failed.";
    }
  }

  return <PublicMessagePage slug={slug} title={title} message={message} />;
}
