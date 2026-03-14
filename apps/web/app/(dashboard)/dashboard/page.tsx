import { redirect } from "next/navigation";
import { getOrganizations } from "@/lib/api-client";

export default async function DashboardIndex() {
  let orgs;

  try {
    orgs = await getOrganizations();
  } catch {
    redirect("/dashboard/onboarding");
  }

  if (orgs.length === 0) {
    redirect("/dashboard/onboarding");
  }

  redirect(`/dashboard/${orgs[0]!.slug}`);
}
