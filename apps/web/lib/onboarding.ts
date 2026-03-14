export function slugifyOrganizationName(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 50);
}

export function validateManagedOnboardingInput(input: {
  name: string;
  serviceName: string;
  createMonitor: boolean;
  monitorUrl: string;
  inviteEmail?: string;
}, currentStep = 6) {
  if (currentStep >= 1 && !input.name.trim()) {
    return "Add an organization name to continue";
  }

  if (currentStep >= 2 && !input.serviceName.trim()) {
    return "Add a first service name to finish setup";
  }

  if (currentStep >= 3 && input.createMonitor && !input.monitorUrl.trim()) {
    return "Add a monitor URL or turn off first-monitor setup";
  }

  if (
    currentStep >= 4 &&
    input.inviteEmail &&
    input.inviteEmail.trim() &&
    !input.inviteEmail.includes("@")
  ) {
    return "Use a valid teammate email address or leave it blank";
  }

  return null;
}

export function nextOnboardingStep(currentStep: number, totalSteps: number) {
  return Math.min(currentStep + 1, totalSteps);
}

export function previousOnboardingStep(currentStep: number) {
  return Math.max(currentStep - 1, 1);
}

export function buildSetupChecklist(input: {
  hasService: boolean;
  hasMonitor: boolean;
  hasSubscriberOrInvite: boolean;
  hasCustomDomain: boolean;
  upgradedPlan: boolean;
}) {
  return [
    {
      key: "service",
      label: "First service created",
      complete: input.hasService,
      optional: false,
    },
    {
      key: "monitor",
      label: "First monitor created",
      complete: input.hasMonitor,
      optional: false,
    },
    {
      key: "audience",
      label: "Subscriber or teammate added",
      complete: input.hasSubscriberOrInvite,
      optional: false,
    },
    {
      key: "custom-domain",
      label: "Custom domain configured",
      complete: input.hasCustomDomain,
      optional: true,
    },
    {
      key: "upgrade",
      label: "Billing upgrade reviewed",
      complete: input.upgradedPlan,
      optional: true,
    },
  ];
}
