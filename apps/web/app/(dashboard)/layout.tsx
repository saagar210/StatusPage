import { auth } from "@/lib/auth";
import { redirect } from "next/navigation";
import { getOrganizations } from "@/lib/api-client";

export default async function DashboardRootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const session = await auth();
  if (!session) {
    redirect("/login");
  }

  // Check if user has any orgs
  try {
    const orgs = await getOrganizations();
    if (orgs.length === 0) {
      // Don't redirect if already on onboarding
      // The middleware handles this via pathname check
    }
  } catch {
    // API might not be running yet - show page anyway
  }

  return <>{children}</>;
}
