import { unsubscribePublicSubscriber } from "@/lib/api-client";
import { PublicMessagePage } from "@/components/status/public-message-page";

interface Props {
  params: Promise<{ slug: string }>;
  searchParams: Promise<{ token?: string }>;
}

export default async function UnsubscribeSubscriberPage({
  params,
  searchParams,
}: Props) {
  const { slug } = await params;
  const { token } = await searchParams;

  let message = "Unsubscribe link is missing.";
  let title = "Unable to unsubscribe";

  if (token) {
    try {
      const response = await unsubscribePublicSubscriber(slug, token);
      message = response.message;
      title = "Unsubscribed";
    } catch (error) {
      message = error instanceof Error ? error.message : "Unsubscribe failed.";
    }
  }

  return <PublicMessagePage slug={slug} title={title} message={message} />;
}
