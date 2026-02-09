import { redirect } from "next/navigation";
import { getOrganizations } from "@/lib/api-client";

export default async function DashboardIndex() {
  try {
    const orgs = await getOrganizations();
    if (orgs.length === 0) {
      redirect("/dashboard/onboarding");
    }
    // Redirect to first org
    redirect(`/dashboard/${orgs[0]!.slug}`);
  } catch {
    redirect("/dashboard/onboarding");
  }
}
