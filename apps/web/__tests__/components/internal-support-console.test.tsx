import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { InternalSupportConsole } from "@/components/admin/internal-support-console";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

function mockJsonResponse(data: unknown, ok = true) {
  return Promise.resolve(
    new Response(JSON.stringify(data), {
      status: ok ? 200 : 400,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function supportPayload() {
  return {
    data: {
      organization: {
        id: "org-1",
        name: "Demo",
        slug: "demo",
        plan: "team",
        subscription_status: "active",
        stripe_customer_id: "cus_123",
        stripe_subscription_id: "sub_123",
        billing_email: "billing@example.com",
        custom_domain: "status.example.com",
        custom_domain_verified_at: "2026-03-13T12:00:00Z",
        custom_domain_status: "verified",
        downgrade_target_plan: "free",
        downgrade_grace_ends_at: "2026-03-27T12:00:00Z",
        downgrade_state: "pending_customer_action",
        member_count: 3,
        pending_invitation_count: 1,
        subscriber_count: 12,
        webhook_count: 2,
      },
      queue_health: {
        pending_email_deliveries: 1,
        failed_email_deliveries: 1,
        pending_webhook_deliveries: 0,
        failed_webhook_deliveries: 1,
      },
      entitlement_violations: [
        {
          code: "monitor_limit",
          message: "This organization has too many monitors for the target plan.",
          current_count: 5,
          allowed_count: 3,
        },
      ],
      required_actions: ["Reduce active monitors before the grace period ends."],
      invitations: [
        {
          id: "invite-1",
          org_id: "org-1",
          email: "teammate@example.com",
          role: "member",
          invited_by: "user-1",
          token: "invite-token",
          expires_at: "2026-03-20T12:00:00Z",
          accepted_at: null,
          canceled_at: null,
          last_sent_at: "2026-03-13T12:00:00Z",
          created_at: "2026-03-13T12:00:00Z",
          updated_at: "2026-03-13T12:00:00Z",
          inviter_name: "Owner",
          inviter_email: "owner@example.com",
          delivery_status: "pending",
        },
      ],
      recent_billing_events: [],
      failed_email_deliveries: [
        {
          id: "email-1",
          notification_type: "incident_update",
          recipient_type: "subscriber",
          recipient_email: "ops@example.com",
          subject: "Incident update",
          status: "failed",
          error_message: "Mailbox unavailable",
          attempt_count: 3,
          max_attempts: 5,
          sent_at: null,
          next_retry_at: null,
          created_at: "2026-03-13T12:00:00Z",
        },
      ],
      failed_webhook_deliveries: [
        {
          id: "webhook-1",
          webhook_config_id: "config-1",
          webhook_name: "Ops Bridge",
          webhook_url: "https://hooks.example.com",
          event_type: "incident.created",
          status: "failed",
          response_status_code: 500,
          error_message: "500 from receiver",
          attempt_count: 2,
          max_attempts: 5,
          next_retry_at: null,
          delivered_at: null,
          created_at: "2026-03-13T12:00:00Z",
        },
      ],
      recent_audit_logs: [],
    },
  };
}

describe("InternalSupportConsole", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("renders the empty state before any support lookup runs", () => {
    render(<InternalSupportConsole />);

    expect(screen.getByText("Ready when you are")).toBeInTheDocument();
    expect(
      screen.getByText(/inspect queue pressure, billing lifecycle, invitation state/i),
    ).toBeInTheDocument();
  });

  it("keeps the load action disabled until token and slug are present", () => {
    render(<InternalSupportConsole />);

    const loadButton = screen.getByRole("button", { name: "Load support view" });
    expect(loadButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Internal admin token"), {
      target: { value: "internal-token" },
    });
    expect(loadButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Organization slug"), {
      target: { value: "demo" },
    });
    expect(loadButton).toBeEnabled();
  });

  it("shows a loading state while support data is being fetched", async () => {
    let resolveQueue!: (value: Response) => void;
    let resolveSupport!: (value: Response) => void;
    const fetchMock = vi.mocked(fetch);
    fetchMock
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveQueue = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveSupport = resolve;
          }),
      );

    render(<InternalSupportConsole />);

    fireEvent.change(screen.getByLabelText("Internal admin token"), {
      target: { value: "internal-token" },
    });
    fireEvent.change(screen.getByLabelText("Organization slug"), {
      target: { value: "demo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Load support view" }));

    expect(
      await screen.findByRole("button", { name: "Loading support data..." }),
    ).toBeDisabled();

    resolveQueue(
      await mockJsonResponse({
        data: {
          pending_email_deliveries: 0,
          failed_email_deliveries: 0,
          pending_webhook_deliveries: 0,
          failed_webhook_deliveries: 0,
          recent_billing_events: 0,
          pending_invitation_emails: 0,
          pending_downgrade_warnings: 0,
          organizations_in_grace: 0,
        },
      }),
    );
    resolveSupport(await mockJsonResponse(supportPayload()));

    await waitFor(() => {
      expect(screen.getByText("Organization snapshot")).toBeInTheDocument();
    });
  });

  it("shows an error state when the lookup fails", async () => {
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockImplementationOnce(() =>
      mockJsonResponse({ error: { message: "Token rejected" } }, false),
    );

    render(<InternalSupportConsole />);

    fireEvent.change(screen.getByLabelText("Internal admin token"), {
      target: { value: "bad-token" },
    });
    fireEvent.change(screen.getByLabelText("Organization slug"), {
      target: { value: "demo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Load support view" }));

    expect(
      await screen.findByText("Could not load support data"),
    ).toBeInTheDocument();
    expect(screen.getByText("Token rejected")).toBeInTheDocument();
  });

  it("renders organization, downgrade, and activity details after a successful lookup", async () => {
    const fetchMock = vi.mocked(fetch);
    fetchMock
      .mockImplementationOnce(() =>
        mockJsonResponse({
          data: {
            pending_email_deliveries: 4,
            failed_email_deliveries: 1,
            pending_webhook_deliveries: 2,
            failed_webhook_deliveries: 1,
            recent_billing_events: 3,
            pending_invitation_emails: 1,
            pending_downgrade_warnings: 1,
            organizations_in_grace: 2,
          },
        }),
      )
      .mockImplementationOnce(() => mockJsonResponse(supportPayload()));

    render(<InternalSupportConsole />);

    fireEvent.change(screen.getByLabelText("Internal admin token"), {
      target: { value: "internal-token" },
    });
    fireEvent.change(screen.getByLabelText("Organization slug"), {
      target: { value: "demo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Load support view" }));

    await waitFor(() => {
      expect(screen.getByText("Organization snapshot")).toBeInTheDocument();
    });

    expect(screen.getByText("Entitlement violations")).toBeInTheDocument();
    expect(screen.getByText(/Reduce active monitors before the grace period ends/i)).toBeInTheDocument();
    expect(screen.getByText("Invitation state")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sync billing" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry email" })).toBeInTheDocument();
  });
});
