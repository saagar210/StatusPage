"use client";

import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  buildSetupChecklist,
  nextOnboardingStep,
  previousOnboardingStep,
  slugifyOrganizationName,
  validateManagedOnboardingInput,
} from "@/lib/onboarding";
import { toast } from "sonner";

const TOTAL_STEPS = 6;

const STEP_TITLES = [
  "Organization",
  "First service",
  "First monitor",
  "Audience",
  "Billing",
  "Review",
] as const;

export default function OnboardingPage() {
  const router = useRouter();
  const [step, setStep] = useState(1);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [serviceName, setServiceName] = useState("API");
  const [monitorUrl, setMonitorUrl] = useState("");
  const [createMonitor, setCreateMonitor] = useState(true);
  const [inviteEmail, setInviteEmail] = useState("");
  const [openBillingAfterSetup, setOpenBillingAfterSetup] = useState(false);
  const [loading, setLoading] = useState(false);

  const previewSlug = useMemo(
    () => slug.trim() || slugifyOrganizationName(name) || "your-org",
    [name, slug],
  );
  const checklist = buildSetupChecklist({
    hasService: Boolean(serviceName.trim()),
    hasMonitor: !createMonitor || Boolean(monitorUrl.trim()),
    hasSubscriberOrInvite: Boolean(inviteEmail.trim()),
    hasCustomDomain: false,
    upgradedPlan: openBillingAfterSetup,
  });

  function goNext() {
    const validationError = validateManagedOnboardingInput({
      name,
      serviceName,
      createMonitor,
      monitorUrl,
      inviteEmail,
    }, step);
    if (validationError) {
      toast.error(validationError);
      return;
    }
    setStep((current) => nextOnboardingStep(current, TOTAL_STEPS));
  }

  function goBack() {
    setStep((current) => previousOnboardingStep(current));
  }

  async function handleSubmit() {
    const validationError = validateManagedOnboardingInput({
      name,
      serviceName,
      createMonitor,
      monitorUrl,
      inviteEmail,
    });
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setLoading(true);
    try {
      const orgRes = await fetch("/api/proxy/api/organizations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          slug: slug.trim() || undefined,
        }),
      });

      if (!orgRes.ok) {
        const body = await orgRes.json();
        throw new Error(body.error?.message || "Failed to create organization");
      }

      const { data: org } = await orgRes.json();

      const serviceRes = await fetch(`/api/proxy/api/organizations/${org.slug}/services`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: serviceName.trim(),
          is_visible: true,
        }),
      });

      if (!serviceRes.ok) {
        const body = await serviceRes.json();
        throw new Error(body.error?.message || "Failed to create first service");
      }

      const { data: service } = await serviceRes.json();

      if (createMonitor) {
        const monitorRes = await fetch(`/api/proxy/api/organizations/${org.slug}/monitors`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            service_id: service.id,
            monitor_type: "http",
            config: {
              type: "http",
              url: monitorUrl.trim(),
            },
            interval_seconds: 60,
            timeout_ms: 5000,
            failure_threshold: 3,
          }),
        });

        if (!monitorRes.ok) {
          const body = await monitorRes.json();
          throw new Error(body.error?.message || "Failed to create first monitor");
        }
      }

      if (inviteEmail.trim()) {
        const invitationRes = await fetch(`/api/proxy/api/organizations/${org.slug}/invitations`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            email: inviteEmail.trim(),
            role: "member",
          }),
        });

        if (!invitationRes.ok) {
          const body = await invitationRes.json();
          throw new Error(body.error?.message || "Failed to invite teammate");
        }
      }

      toast.success("Your managed workspace is ready");
      router.push(
        openBillingAfterSetup
          ? `/dashboard/${org.slug}/settings?billing=upgrade`
          : `/dashboard/${org.slug}`,
      );
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="mx-auto flex min-h-screen w-full max-w-6xl items-center justify-center px-4 py-12">
      <Card className="w-full overflow-hidden border-0 shadow-none">
        <div className="grid gap-8 lg:grid-cols-[1.15fr_0.85fr]">
          <div className="space-y-6 rounded-3xl border bg-background p-6 md:p-8">
            <div className="space-y-4">
              <div className="flex flex-wrap gap-2">
                {STEP_TITLES.map((title, index) => {
                  const stepNumber = index + 1;
                  const complete = stepNumber < step;
                  const active = stepNumber === step;
                  return (
                    <Badge
                      key={title}
                      variant={active || complete ? "secondary" : "outline"}
                      className="gap-2 px-3 py-1"
                    >
                      <span>{stepNumber}</span>
                      <span>{title}</span>
                    </Badge>
                  );
                })}
              </div>
              <div>
                <h1 className="text-3xl font-semibold leading-none">
                  Create your managed workspace
                </h1>
                <CardDescription className="mt-2 text-sm">
                  This guided flow gets you from sign-in to a real public status page,
                  first monitor, and optional teammate invite without needing the docs.
                </CardDescription>
              </div>
            </div>

            {step === 1 ? (
              <StepCard
                title="Step 1: Organization identity"
                description="Choose the customer-facing name and public slug for your status page."
              >
                <Field>
                  <Label htmlFor="name">Organization name</Label>
                  <Input
                    id="name"
                    placeholder="Acme Cloud"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    required
                  />
                </Field>
                <Field>
                  <Label htmlFor="slug">Status page slug</Label>
                  <Input
                    id="slug"
                    placeholder="acme-cloud"
                    value={slug}
                    onChange={(e) => setSlug(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">Preview: `/s/{previewSlug}`</p>
                </Field>
              </StepCard>
            ) : null}

            {step === 2 ? (
              <StepCard
                title="Step 2: First service"
                description="Create the first visible service customers will see on the public page."
              >
                <Field>
                  <Label htmlFor="service-name">First service</Label>
                  <Input
                    id="service-name"
                    placeholder="API"
                    value={serviceName}
                    onChange={(e) => setServiceName(e.target.value)}
                    required
                  />
                </Field>
              </StepCard>
            ) : null}

            {step === 3 ? (
              <StepCard
                title="Step 3: First monitor"
                description="Start with a single HTTP monitor now, or skip and add it later."
              >
                <label className="flex items-center justify-between gap-3 rounded-lg border p-4 text-sm">
                  <span>Create the first HTTP monitor now</span>
                  <input
                    type="checkbox"
                    checked={createMonitor}
                    onChange={(e) => setCreateMonitor(e.target.checked)}
                    className="h-4 w-4"
                  />
                </label>
                <Field>
                  <Label htmlFor="monitor-url">Monitor URL</Label>
                  <Input
                    id="monitor-url"
                    type="url"
                    placeholder="https://api.example.com/health"
                    value={monitorUrl}
                    onChange={(e) => setMonitorUrl(e.target.value)}
                    disabled={!createMonitor}
                  />
                </Field>
              </StepCard>
            ) : null}

            {step === 4 ? (
              <StepCard
                title="Step 4: Notifications and teammate invite"
                description="Invite one teammate now if you want shared ownership from day one."
              >
                <Field>
                  <Label htmlFor="invite-email">Teammate email</Label>
                  <Input
                    id="invite-email"
                    type="email"
                    placeholder="teammate@example.com"
                    value={inviteEmail}
                    onChange={(e) => setInviteEmail(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">
                    They will receive an invite email and must accept with the matching GitHub account.
                  </p>
                </Field>
              </StepCard>
            ) : null}

            {step === 5 ? (
              <StepCard
                title="Step 5: Billing prompt"
                description="Upgrades stay optional. Free includes 3 monitors and the default hosted URL."
              >
                <label className="flex items-center justify-between gap-3 rounded-lg border p-4 text-sm">
                  <span>Open billing setup after the workspace is created</span>
                  <input
                    type="checkbox"
                    checked={openBillingAfterSetup}
                    onChange={(e) => setOpenBillingAfterSetup(e.target.checked)}
                    className="h-4 w-4"
                  />
                </label>
              </StepCard>
            ) : null}

            {step === 6 ? (
              <StepCard
                title="Step 6: Review and launch"
                description="This is the final checklist before the workspace is created."
              >
                <div className="grid gap-3 rounded-lg border bg-muted/30 p-4 text-sm">
                  <SummaryRow label="Workspace" value={name.trim() || "Not set"} />
                  <SummaryRow label="Public page" value={`/s/${previewSlug}`} />
                  <SummaryRow label="First service" value={serviceName.trim() || "Not set"} />
                  <SummaryRow
                    label="First monitor"
                    value={createMonitor ? monitorUrl.trim() || "Not set" : "Skip for now"}
                  />
                  <SummaryRow
                    label="Teammate invite"
                    value={inviteEmail.trim() || "Skip for now"}
                  />
                  <SummaryRow
                    label="Next destination"
                    value={openBillingAfterSetup ? "Billing setup" : "Dashboard overview"}
                  />
                </div>
              </StepCard>
            ) : null}

            <div className="flex flex-wrap gap-3">
              <Button type="button" variant="outline" onClick={goBack} disabled={step === 1 || loading}>
                Back
              </Button>
              {step < TOTAL_STEPS ? (
                <Button type="button" onClick={goNext} disabled={loading}>
                  Continue
                </Button>
              ) : (
                <Button type="button" onClick={handleSubmit} disabled={loading}>
                  {loading ? "Creating workspace..." : "Create Managed Workspace"}
                </Button>
              )}
            </div>
          </div>

          <div className="space-y-6 rounded-[2rem] border bg-muted/30 p-6 md:p-8">
            <div className="space-y-3">
              <p className="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                Success checklist
              </p>
              <div className="space-y-3">
                {checklist.map((item) => (
                  <div
                    key={item.key}
                    className="flex items-center justify-between rounded-xl border bg-background px-4 py-3"
                  >
                    <div>
                      <p className="text-sm font-medium">{item.label}</p>
                      <p className="text-xs text-muted-foreground">
                        {item.optional ? "Optional" : "Required for launch"}
                      </p>
                    </div>
                    <Badge variant={item.complete ? "secondary" : "outline"}>
                      {item.complete ? "Ready" : item.optional ? "Optional" : "Pending"}
                    </Badge>
                  </div>
                ))}
              </div>
            </div>

            <div className="space-y-3 rounded-2xl border bg-background p-5">
              <p className="text-sm font-semibold">Public page preview</p>
              <div className="space-y-2">
                <p className="text-xl font-semibold">{name.trim() || "Your organization"}</p>
                <p className="text-sm text-muted-foreground">https://statuspage.sh/s/{previewSlug}</p>
              </div>
              <div className="rounded-xl border border-dashed p-4 text-sm text-muted-foreground">
                {serviceName.trim() || "Your first service"} will appear here first.
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}

function StepCard({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-xl font-semibold">{title}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
      <div className="space-y-4">{children}</div>
    </div>
  );
}

function Field({ children }: { children: ReactNode }) {
  return <div className="space-y-2">{children}</div>;
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-right font-medium">{value}</span>
    </div>
  );
}
