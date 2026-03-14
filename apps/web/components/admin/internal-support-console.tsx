"use client";

import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import type {
  Invitation,
  SupportOrganizationPayload,
  SupportQueueHealth,
  SupportSearchResult,
} from "@/lib/types";
import {
  formatDowngradeState,
  formatOrganizationPlan,
  formatSubscriptionStatus,
} from "@/lib/types";

const EMPTY_QUEUE_HEALTH: SupportQueueHealth = {
  pending_email_deliveries: 0,
  failed_email_deliveries: 0,
  pending_webhook_deliveries: 0,
  failed_webhook_deliveries: 0,
  recent_billing_events: 0,
  pending_invitation_emails: 0,
  pending_downgrade_warnings: 0,
  organizations_in_grace: 0,
};

async function parseErrorMessage(response: Response, fallback: string) {
  const body = (await response.json().catch(() => null)) as
    | { error?: { message?: string } }
    | null;
  return body?.error?.message || fallback;
}

export function InternalSupportConsole() {
  const [adminToken, setAdminToken] = useState("");
  const [orgSlug, setOrgSlug] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SupportSearchResult[]>([]);
  const [queueHealth, setQueueHealth] = useState<SupportQueueHealth | null>(null);
  const [support, setSupport] = useState<SupportOrganizationPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionState, setActionState] = useState<string | null>(null);
  const canLoad = adminToken.trim().length > 0 && orgSlug.trim().length > 0 && !loading;
  const canSearch = adminToken.trim().length > 0 && searchQuery.trim().length > 1 && !loading;

  const platformPressure = useMemo(() => {
    const current = queueHealth ?? EMPTY_QUEUE_HEALTH;
    return current.pending_email_deliveries + current.pending_webhook_deliveries;
  }, [queueHealth]);

  async function authenticatedFetch(path: string, init?: RequestInit) {
    return fetch(path, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        "x-statuspage-admin-token": adminToken.trim(),
        ...(init?.headers ?? {}),
      },
    });
  }

  async function loadSupportData(slugOverride?: string) {
    const activeSlug = slugOverride ?? orgSlug.trim();
    if (!adminToken.trim() || !activeSlug) {
      return;
    }

    setLoading(true);
    setError(null);
    setOrgSlug(activeSlug);

    try {
      const [queueRes, supportRes] = await Promise.all([
        authenticatedFetch("/api/proxy/api/admin/queue-health", { cache: "no-store" }),
        authenticatedFetch(`/api/proxy/api/admin/organizations/${activeSlug}/support`, {
          cache: "no-store",
        }),
      ]);

      if (!queueRes.ok) {
        throw new Error(await parseErrorMessage(queueRes, "Failed to load queue health"));
      }
      if (!supportRes.ok) {
        throw new Error(
          await parseErrorMessage(supportRes, "Failed to load organization support data"),
        );
      }

      const queueBody = (await queueRes.json()) as { data: SupportQueueHealth };
      const supportBody = (await supportRes.json()) as { data: SupportOrganizationPayload };
      setQueueHealth(queueBody.data);
      setSupport(supportBody.data);
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Failed to load internal support data";
      setError(message);
      setQueueHealth(null);
      setSupport(null);
    } finally {
      setLoading(false);
    }
  }

  async function searchOrganizations() {
    if (!canSearch) {
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(
        `/api/proxy/api/admin/organizations/search?q=${encodeURIComponent(searchQuery.trim())}`,
        { cache: "no-store" },
      );
      if (!res.ok) {
        throw new Error(await parseErrorMessage(res, "Failed to search organizations"));
      }
      const body = (await res.json()) as { data: SupportSearchResult[] };
      setSearchResults(body.data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to search organizations");
      setSearchResults([]);
    } finally {
      setLoading(false);
    }
  }

  async function postAdminAction(path: string, successMessage: string) {
    setActionState(path);
    try {
      const res = await authenticatedFetch(path, { method: "POST" });
      if (!res.ok) {
        throw new Error(await parseErrorMessage(res, "Action failed"));
      }
      const body = (await res.json()) as { data?: { message?: string } };
      toast.success(body.data?.message || successMessage);
      await loadSupportData();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Action failed");
    } finally {
      setActionState(null);
    }
  }

  async function retryDelivery(kind: "email" | "webhook", id: string) {
    if (!support) return;
    const path =
      kind === "email"
        ? `/api/proxy/api/admin/organizations/${support.organization.slug}/retry/email/${id}`
        : `/api/proxy/api/admin/organizations/${support.organization.slug}/retry/webhook/${id}`;
    await postAdminAction(path, "Retry queued");
  }

  async function resendInvitation(invitation: Invitation) {
    if (!support) return;
    await postAdminAction(
      `/api/proxy/api/admin/organizations/${support.organization.slug}/invitations/${invitation.id}/resend`,
      "Invitation email queued",
    );
  }

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h1 className="text-3xl font-bold">Internal Support</h1>
        <p className="text-sm text-muted-foreground">
          Search managed organizations, inspect downgrade and billing state, and take common support actions without leaving the dashboard.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Operator Access</CardTitle>
          <CardDescription>
            Enter the internal admin token, then search or load a specific organization.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-[1fr_1fr_1fr_auto]">
          <Field>
            <Label htmlFor="admin-token">Internal admin token</Label>
            <Input
              id="admin-token"
              type="password"
              value={adminToken}
              onChange={(event) => setAdminToken(event.target.value)}
              placeholder="internal-admin-token"
            />
          </Field>
          <Field>
            <Label htmlFor="org-slug">Organization slug</Label>
            <Input
              id="org-slug"
              value={orgSlug}
              onChange={(event) => setOrgSlug(event.target.value)}
              placeholder="demo"
            />
          </Field>
          <Field>
            <Label htmlFor="org-search">Search organizations</Label>
            <Input
              id="org-search"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="slug, name, billing email, Stripe ID"
            />
          </Field>
          <div className="flex items-end gap-2">
            <Button type="button" onClick={() => loadSupportData()} disabled={!canLoad}>
              {loading ? "Loading support data..." : "Load support view"}
            </Button>
            <Button type="button" variant="outline" onClick={searchOrganizations} disabled={!canSearch}>
              Search
            </Button>
          </div>
        </CardContent>
      </Card>

      {searchResults.length > 0 ? (
        <Card>
          <CardHeader>
            <CardTitle>Search results</CardTitle>
            <CardDescription>
              Choose an organization to load the detailed support view.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3">
            {searchResults.map((result) => (
              <button
                key={result.id}
                type="button"
                className="rounded-xl border p-4 text-left transition hover:border-foreground/40"
                onClick={() => loadSupportData(result.slug)}
              >
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium">{result.name}</span>
                  <Badge variant="outline">{result.slug}</Badge>
                  <Badge variant="secondary">{formatOrganizationPlan(result.plan)}</Badge>
                  <Badge variant="outline">{formatDowngradeState(result.downgrade_state)}</Badge>
                </div>
                <p className="mt-2 text-sm text-muted-foreground">
                  {result.billing_email || "No billing email"} • {formatSubscriptionStatus(result.subscription_status)}
                </p>
              </button>
            ))}
          </CardContent>
        </Card>
      ) : null}

      {!loading && !support && !error && (
        <Card>
          <CardHeader>
            <CardTitle>Ready when you are</CardTitle>
            <CardDescription>
              Add a token and load a slug to inspect queue pressure, billing lifecycle, invitation state, and recent failures.
            </CardDescription>
          </CardHeader>
        </Card>
      )}

      {error ? (
        <Card className="border-destructive/50">
          <CardHeader>
            <CardTitle>Could not load support data</CardTitle>
            <CardDescription>{error}</CardDescription>
          </CardHeader>
        </Card>
      ) : null}

      {support ? (
        <>
          <div className="grid gap-4 lg:grid-cols-4">
            <SupportStatCard
              title="Plan"
              value={formatOrganizationPlan(support.organization.plan)}
              detail={formatSubscriptionStatus(support.organization.subscription_status)}
            />
            <SupportStatCard
              title="Downgrade"
              value={formatDowngradeState(support.organization.downgrade_state)}
              detail={
                support.organization.downgrade_target_plan
                  ? `Target ${formatOrganizationPlan(support.organization.downgrade_target_plan)}`
                  : "No pending change"
              }
            />
            <SupportStatCard
              title="Members"
              value={String(support.organization.member_count)}
              detail={`${support.organization.pending_invitation_count} pending invites`}
            />
            <SupportStatCard
              title="Queue pressure"
              value={String(platformPressure)}
              detail={`${queueHealth?.organizations_in_grace ?? 0} orgs in grace`}
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
            <Card>
              <CardHeader>
                <CardTitle>Organization snapshot</CardTitle>
                <CardDescription>Core state for {support.organization.name}.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3 text-sm">
                <SnapshotRow label="Slug" value={support.organization.slug} />
                <SnapshotRow label="Billing email" value={support.organization.billing_email || "Not set"} />
                <SnapshotRow label="Stripe customer" value={support.organization.stripe_customer_id || "Not linked"} />
                <SnapshotRow label="Stripe subscription" value={support.organization.stripe_subscription_id || "Not linked"} />
                <SnapshotRow label="Custom domain" value={support.organization.custom_domain || "Not configured"} />
                <SnapshotRow label="Domain status" value={support.organization.custom_domain_status} />
                <SnapshotRow label="Grace ends" value={support.organization.downgrade_grace_ends_at || "Not scheduled"} />
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Operator actions</CardTitle>
                <CardDescription>
                  Use these only when normal customer flows or background workers need help.
                </CardDescription>
              </CardHeader>
              <CardContent className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    postAdminAction(
                      `/api/proxy/api/admin/organizations/${support.organization.slug}/billing/sync`,
                      "Billing synced",
                    )
                  }
                  disabled={actionState !== null}
                >
                  {actionState?.includes("/billing/sync") ? "Syncing..." : "Sync billing"}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    postAdminAction(
                      `/api/proxy/api/admin/organizations/${support.organization.slug}/downgrade/enforce`,
                      "Downgrade enforced",
                    )
                  }
                  disabled={actionState !== null}
                >
                  {actionState?.includes("/downgrade/enforce") ? "Enforcing..." : "Force enforce"}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    postAdminAction(
                      `/api/proxy/api/admin/organizations/${support.organization.slug}/downgrade/cancel`,
                      "Downgrade canceled",
                    )
                  }
                  disabled={actionState !== null}
                >
                  {actionState?.includes("/downgrade/cancel") ? "Canceling..." : "Cancel downgrade"}
                </Button>
              </CardContent>
            </Card>
          </div>

          {support.entitlement_violations.length > 0 ? (
            <Card>
              <CardHeader>
                <CardTitle>Entitlement violations</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {support.entitlement_violations.map((violation) => (
                  <div key={violation.code} className="rounded-lg border p-4 text-sm">
                    <p className="font-medium">{violation.message}</p>
                  </div>
                ))}
                {support.required_actions.length > 0 ? (
                  <div className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
                    {support.required_actions.map((action) => (
                      <p key={action}>- {action}</p>
                    ))}
                  </div>
                ) : null}
              </CardContent>
            </Card>
          ) : null}

          <Card>
            <CardHeader>
              <CardTitle>Invitation state</CardTitle>
              <CardDescription>
                Track pending invites and resend delivery when needed.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {support.invitations.length === 0 ? (
                <p className="text-sm text-muted-foreground">No invitations found.</p>
              ) : (
                support.invitations.map((invitation) => (
                  <div key={invitation.id} className="rounded-lg border p-4">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-medium">{invitation.email}</p>
                      <Badge variant="outline">{invitation.role}</Badge>
                      <Badge variant="secondary">{invitation.delivery_status}</Badge>
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">
                      Expires {invitation.expires_at}
                    </p>
                    <div className="mt-3 flex flex-wrap gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        onClick={() => resendInvitation(invitation)}
                        disabled={actionState !== null || invitation.delivery_status !== "pending"}
                      >
                        {actionState?.includes(`/invitations/${invitation.id}/resend`)
                          ? "Resending..."
                          : "Resend invite"}
                      </Button>
                    </div>
                  </div>
                ))
              )}
            </CardContent>
          </Card>

          <div className="grid gap-4 xl:grid-cols-2">
            <ActivityCard
              title="Failed email deliveries"
              items={support.failed_email_deliveries.map((entry) => ({
                id: entry.id,
                title: entry.recipient_email,
                detail: entry.error_message || "Unknown email error",
                actionLabel: "Retry email",
                onAction: () => retryDelivery("email", entry.id),
                disabled: actionState !== null,
              }))}
            />
            <ActivityCard
              title="Failed webhook deliveries"
              items={support.failed_webhook_deliveries.map((entry) => ({
                id: entry.id,
                title: entry.webhook_name,
                detail: entry.error_message || "Unknown webhook error",
                actionLabel: "Retry webhook",
                onAction: () => retryDelivery("webhook", entry.id),
                disabled: actionState !== null,
              }))}
            />
          </div>
        </>
      ) : null}
    </div>
  );
}

function Field({ children }: { children: ReactNode }) {
  return <div className="space-y-2">{children}</div>;
}

function SupportStatCard({
  title,
  value,
  detail,
}: {
  title: string;
  value: string;
  detail: string;
}) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        <p className="text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function SnapshotRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-right font-medium">{value}</span>
    </div>
  );
}

function ActivityCard({
  title,
  items,
}: {
  title: string;
  items: Array<{
    id: string;
    title: string;
    detail: string;
    actionLabel: string;
    onAction: () => void;
    disabled: boolean;
  }>;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {items.length === 0 ? (
          <p className="text-sm text-muted-foreground">No failures right now.</p>
        ) : (
          items.map((item) => (
            <div key={item.id} className="rounded-lg border p-4">
              <p className="font-medium">{item.title}</p>
              <p className="mt-1 text-sm text-muted-foreground">{item.detail}</p>
              <Button
                type="button"
                variant="outline"
                className="mt-3"
                onClick={item.onAction}
                disabled={item.disabled}
              >
                {item.actionLabel}
              </Button>
            </div>
          ))
        )}
      </CardContent>
    </Card>
  );
}
