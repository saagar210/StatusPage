import { describe, expect, it } from "vitest";

import {
  buildSetupChecklist,
  nextOnboardingStep,
  previousOnboardingStep,
  slugifyOrganizationName,
  validateManagedOnboardingInput,
} from "@/lib/onboarding";

describe("managed onboarding helpers", () => {
  it("slugifies an organization name into a public-page-safe slug", () => {
    expect(slugifyOrganizationName(" Acme Cloud API ")).toBe("acme-cloud-api");
  });

  it("requires an organization name before onboarding can continue", () => {
    expect(
      validateManagedOnboardingInput({
        name: "   ",
        serviceName: "API",
        createMonitor: false,
        monitorUrl: "",
        inviteEmail: "",
      }),
    ).toBe("Add an organization name to continue");
  });

  it("allows step one to continue before later-step fields are filled", () => {
    expect(
      validateManagedOnboardingInput(
        {
          name: "Acme",
          serviceName: " ",
          createMonitor: true,
          monitorUrl: "",
          inviteEmail: "not-an-email",
        },
        1,
      ),
    ).toBeNull();
  });

  it("requires a service name before onboarding can continue", () => {
    expect(
      validateManagedOnboardingInput(
        {
          name: "Acme",
          serviceName: " ",
          createMonitor: false,
          monitorUrl: "",
          inviteEmail: "",
        },
        2,
      ),
    ).toBe("Add a first service name to finish setup");
  });

  it("requires a monitor URL only when monitor creation is enabled", () => {
    expect(
      validateManagedOnboardingInput(
        {
          name: "Acme",
          serviceName: "API",
          createMonitor: true,
          monitorUrl: " ",
          inviteEmail: "",
        },
        3,
      ),
    ).toBe("Add a monitor URL or turn off first-monitor setup");

    expect(
      validateManagedOnboardingInput(
        {
          name: "Acme",
          serviceName: "API",
          createMonitor: false,
          monitorUrl: " ",
          inviteEmail: "",
        },
        3,
      ),
    ).toBeNull();
  });

  it("rejects malformed teammate emails and allows blank values", () => {
    expect(
      validateManagedOnboardingInput(
        {
          name: "Acme",
          serviceName: "API",
          createMonitor: false,
          monitorUrl: "",
          inviteEmail: "not-an-email",
        },
        4,
      ),
    ).toBe("Use a valid teammate email address or leave it blank");

    expect(
      validateManagedOnboardingInput(
        {
          name: "Acme",
          serviceName: "API",
          createMonitor: false,
          monitorUrl: "",
          inviteEmail: "",
        },
        4,
      ),
    ).toBeNull();
  });

  it("moves onboarding steps within bounds", () => {
    expect(nextOnboardingStep(1, 6)).toBe(2);
    expect(nextOnboardingStep(6, 6)).toBe(6);
    expect(previousOnboardingStep(1)).toBe(1);
    expect(previousOnboardingStep(4)).toBe(3);
  });

  it("builds a setup checklist with required and optional items", () => {
    const checklist = buildSetupChecklist({
      hasService: true,
      hasMonitor: false,
      hasSubscriberOrInvite: true,
      hasCustomDomain: false,
      upgradedPlan: false,
    });

    expect(checklist).toHaveLength(5);
    expect(checklist[0]).toMatchObject({ complete: true, optional: false });
    expect(checklist[1]).toMatchObject({ complete: false, optional: false });
    expect(checklist[3]).toMatchObject({ optional: true });
  });
});
