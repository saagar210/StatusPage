"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type {
  BillingSummary,
  Invitation,
  MemberRole,
  MemberWithUser,
  NotificationLogEntry,
  NotificationPreferences,
  Organization,
  OrganizationPlan,
  SubscriberListItem,
  WebhookConfig,
  WebhookDeliveryEntry,
} from "@/lib/types";
import {
  formatDowngradeState,
  PLAN_FEATURES,
  formatOrganizationPlan,
  formatPlanMonitorLimit,
  formatSubscriptionStatus,
} from "@/lib/types";
import { toast } from "sonner";

const WEBHOOK_EVENT_OPTIONS = [
  { value: "incident.created", label: "Incident created" },
  { value: "incident.updated", label: "Incident updated" },
  { value: "incident.resolved", label: "Incident resolved" },
  { value: "service.status_changed", label: "Service status changed" },
] as const;

const DEFAULT_PREFERENCES: NotificationPreferences = {
  id: "",
  org_id: "",
  email_on_incident_created: true,
  email_on_incident_updated: true,
  email_on_incident_resolved: true,
  email_on_service_status_changed: false,
  webhook_on_incident_created: true,
  webhook_on_incident_updated: true,
  webhook_on_incident_resolved: true,
  webhook_on_service_status_changed: true,
  uptime_alert_threshold: 95,
  uptime_alert_enabled: true,
  created_at: "",
  updated_at: "",
};

const DELIVERY_STATUS_OPTIONS = [
  { value: "all", label: "All statuses" },
  { value: "pending", label: "Pending" },
  { value: "sending", label: "Sending" },
  { value: "sent", label: "Sent" },
  { value: "success", label: "Success" },
  { value: "failed", label: "Failed" },
] as const;

const DEFAULT_DELIVERY_PAGINATION = {
  page: 1,
  per_page: 10,
  total: 0,
};

type PreferenceKey =
  | "email_on_incident_created"
  | "email_on_incident_updated"
  | "email_on_incident_resolved"
  | "email_on_service_status_changed"
  | "webhook_on_incident_created"
  | "webhook_on_incident_updated"
  | "webhook_on_incident_resolved"
  | "webhook_on_service_status_changed"
  | "uptime_alert_enabled";

type DeliveryPagination = {
  page: number;
  per_page: number;
  total: number;
};

export default function SettingsPage() {
  const params = useParams<{ slug: string }>();
  const slug = params.slug;
  const [org, setOrg] = useState<Organization | null>(null);
  const [name, setName] = useState("");
  const [brandColor, setBrandColor] = useState("#3B82F6");
  const [timezone, setTimezone] = useState("UTC");
  const [logoUrl, setLogoUrl] = useState("");
  const [customDomain, setCustomDomain] = useState("");
  const [loading, setLoading] = useState(false);
  const [monitorCount, setMonitorCount] = useState<number | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [preferences, setPreferences] =
    useState<NotificationPreferences>(DEFAULT_PREFERENCES);
  const [preferencesLoading, setPreferencesLoading] = useState(false);
  const [webhooks, setWebhooks] = useState<WebhookConfig[]>([]);
  const [webhookLoading, setWebhookLoading] = useState(false);
  const [billingSummary, setBillingSummary] = useState<BillingSummary | null>(
    null,
  );
  const [billingActionState, setBillingActionState] = useState<string | null>(
    null,
  );
  const [members, setMembers] = useState<MemberWithUser[]>([]);
  const [invitations, setInvitations] = useState<Invitation[]>([]);
  const [memberLoading, setMemberLoading] = useState(false);
  const [memberActionState, setMemberActionState] = useState<{
    id: string;
    action: "role" | "remove";
  } | null>(null);
  const [invitationActionState, setInvitationActionState] = useState<{
    id: string;
    action: "copy" | "cancel" | "resend";
  } | null>(null);
  const [domainVerification, setDomainVerification] = useState<{
    loading: boolean;
    message: string | null;
    is_ready: boolean;
    expected_target: string | null;
  }>({
    loading: false,
    message: null,
    is_ready: false,
    expected_target: null,
  });
  const [newMember, setNewMember] = useState<{
    email: string;
    role: MemberRole;
  }>({
    email: "",
    role: "member",
  });
  const [subscribers, setSubscribers] = useState<SubscriberListItem[]>([]);
  const [subscriberActionState, setSubscriberActionState] = useState<{
    id: string;
    action: "delete" | "resend";
  } | null>(null);
  const [emailDeliveries, setEmailDeliveries] = useState<NotificationLogEntry[]>(
    [],
  );
  const [webhookDeliveries, setWebhookDeliveries] = useState<
    WebhookDeliveryEntry[]
  >([]);
  const [emailStatusFilter, setEmailStatusFilter] = useState("all");
  const [webhookStatusFilter, setWebhookStatusFilter] = useState("all");
  const [deliveryPageSize, setDeliveryPageSize] = useState(10);
  const [emailPage, setEmailPage] = useState(1);
  const [webhookPage, setWebhookPage] = useState(1);
  const [emailPagination, setEmailPagination] = useState<DeliveryPagination>(
    DEFAULT_DELIVERY_PAGINATION,
  );
  const [webhookPagination, setWebhookPagination] = useState<DeliveryPagination>(
    DEFAULT_DELIVERY_PAGINATION,
  );
  const [deliveryActionState, setDeliveryActionState] = useState<{
    kind: "email" | "webhook";
    id: string;
  } | null>(null);
  const [newWebhook, setNewWebhook] = useState({
    name: "",
    url: "",
    secret: "",
    event_types: ["incident.created", "incident.updated"],
  });

  useEffect(() => {
    async function loadSettings() {
      setLoadError(null);

      try {
        const emailQuery = buildDeliveryQuery(
          emailPage,
          deliveryPageSize,
          emailStatusFilter,
        );
        const webhookQuery = buildDeliveryQuery(
          webhookPage,
          deliveryPageSize,
          webhookStatusFilter,
        );
        const [
          orgRes,
          monitorRes,
          preferencesRes,
          billingRes,
          webhooksRes,
          membersRes,
          invitationsRes,
          subscribersRes,
          emailDeliveriesRes,
          webhookDeliveriesRes,
        ] = await Promise.all([
          fetch(`/api/proxy/api/organizations/${slug}`),
          fetch(`/api/proxy/api/organizations/${slug}/monitors`),
          fetch(`/api/proxy/api/organizations/${slug}/notifications/preferences`),
          fetch(`/api/proxy/api/organizations/${slug}/billing`),
          fetch(`/api/proxy/api/organizations/${slug}/notifications/webhooks`),
          fetch(`/api/proxy/api/organizations/${slug}/members`),
          fetch(`/api/proxy/api/organizations/${slug}/invitations`),
          fetch(`/api/proxy/api/organizations/${slug}/notifications/subscribers`),
          fetch(`/api/proxy/api/organizations/${slug}/notifications/deliveries/email${emailQuery}`),
          fetch(
            `/api/proxy/api/organizations/${slug}/notifications/deliveries/webhooks${webhookQuery}`,
          ),
        ]);

        if (!orgRes.ok) {
          throw new Error("Failed to load organization settings");
        }
        if (!preferencesRes.ok) {
          throw new Error("Failed to load notification preferences");
        }
        if (!billingRes.ok) {
          throw new Error("Failed to load billing summary");
        }
        if (!webhooksRes.ok) {
          throw new Error("Failed to load webhooks");
        }
        if (!membersRes.ok) {
          throw new Error("Failed to load team members");
        }
        if (!invitationsRes.ok) {
          throw new Error("Failed to load invitations");
        }
        if (!subscribersRes.ok) {
          throw new Error("Failed to load subscribers");
        }
        if (!emailDeliveriesRes.ok) {
          throw new Error("Failed to load email delivery activity");
        }
        if (!webhookDeliveriesRes.ok) {
          throw new Error("Failed to load webhook delivery activity");
        }

        const [
          orgBody,
          preferencesBody,
          billingBody,
          webhooksBody,
          membersBody,
          invitationsBody,
          subscribersBody,
          emailDeliveriesBody,
          webhookDeliveriesBody,
        ] = await Promise.all([
          orgRes.json(),
          preferencesRes.json(),
          billingRes.json(),
          webhooksRes.json(),
          membersRes.json(),
          invitationsRes.json(),
          subscribersRes.json(),
          emailDeliveriesRes.json(),
          webhookDeliveriesRes.json(),
        ]);

        setOrg(orgBody.data);
        setName(orgBody.data.name);
        setBrandColor(orgBody.data.brand_color);
        setTimezone(orgBody.data.timezone);
        setLogoUrl(orgBody.data.logo_url ?? "");
        setCustomDomain(orgBody.data.custom_domain ?? "");
        setPreferences(preferencesBody.data);
        setBillingSummary(billingBody.data);
        setWebhooks(webhooksBody.data);
        setMembers(membersBody.data);
        setInvitations(invitationsBody.data);
        setSubscribers(subscribersBody.data);
        setEmailDeliveries(emailDeliveriesBody.data);
        setWebhookDeliveries(webhookDeliveriesBody.data);
        setEmailPagination(emailDeliveriesBody.pagination ?? DEFAULT_DELIVERY_PAGINATION);
        setWebhookPagination(
          webhookDeliveriesBody.pagination ?? DEFAULT_DELIVERY_PAGINATION,
        );

        if (monitorRes.ok) {
          const monitorBody = await monitorRes.json();
          setMonitorCount(monitorBody.data.length);
        } else {
          setMonitorCount(null);
          toast.error("Failed to load monitor usage");
        }
      } catch (error) {
        setLoadError("Failed to load settings");
        toast.error(
          error instanceof Error ? error.message : "Failed to load settings",
        );
      }
    }

    loadSettings();
  }, [
    deliveryPageSize,
    emailPage,
    emailStatusFilter,
    slug,
    webhookPage,
    webhookStatusFilter,
  ]);

  async function refreshNotificationOperations() {
    const emailQuery = buildDeliveryQuery(
      emailPage,
      deliveryPageSize,
      emailStatusFilter,
    );
    const webhookQuery = buildDeliveryQuery(
      webhookPage,
      deliveryPageSize,
      webhookStatusFilter,
    );
    const [subscribersRes, emailRes, webhookRes] = await Promise.all([
      fetch(`/api/proxy/api/organizations/${slug}/notifications/subscribers`),
      fetch(`/api/proxy/api/organizations/${slug}/notifications/deliveries/email${emailQuery}`),
      fetch(
        `/api/proxy/api/organizations/${slug}/notifications/deliveries/webhooks${webhookQuery}`,
      ),
    ]);

    if (!subscribersRes.ok) {
      throw new Error("Failed to refresh subscribers");
    }
    if (!emailRes.ok) {
      throw new Error("Failed to refresh email delivery activity");
    }
    if (!webhookRes.ok) {
      throw new Error("Failed to refresh webhook delivery activity");
    }

    const [subscribersBody, emailBody, webhookBody] = await Promise.all([
      subscribersRes.json(),
      emailRes.json(),
      webhookRes.json(),
    ]);

    setSubscribers(subscribersBody.data);
    setEmailDeliveries(emailBody.data);
    setWebhookDeliveries(webhookBody.data);
    setEmailPagination(emailBody.pagination ?? DEFAULT_DELIVERY_PAGINATION);
    setWebhookPagination(webhookBody.pagination ?? DEFAULT_DELIVERY_PAGINATION);
  }

  async function handleSaveOrganization(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);

    try {
      const res = await fetch(`/api/proxy/api/organizations/${slug}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          brand_color: brandColor,
          timezone: timezone.trim(),
          logo_url: logoUrl.trim(),
          custom_domain: customDomain.trim(),
        }),
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to update");
      }

      const body = await res.json();
      setOrg(body.data);
      setTimezone(body.data.timezone);
      setLogoUrl(body.data.logo_url ?? "");
      setCustomDomain(body.data.custom_domain ?? "");
      toast.success("Organization settings saved");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  async function startCheckout(plan: OrganizationPlan) {
    setBillingActionState(`checkout:${plan}`);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/billing/checkout`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ plan }),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to start upgrade");
      }

      const body = await res.json();
      window.location.assign(body.data.url);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to start upgrade",
      );
      setBillingActionState(null);
    }
  }

  async function openBillingPortal() {
    setBillingActionState("portal");

    try {
      const res = await fetch(`/api/proxy/api/organizations/${slug}/billing/portal`, {
        method: "POST",
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to open billing portal");
      }

      const body = await res.json();
      window.location.assign(body.data.url);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to open billing portal",
      );
      setBillingActionState(null);
    }
  }

  async function handleSavePreferences(e: React.FormEvent) {
    e.preventDefault();
    setPreferencesLoading(true);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/preferences`,
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            email_on_incident_created: preferences.email_on_incident_created,
            email_on_incident_updated: preferences.email_on_incident_updated,
            email_on_incident_resolved: preferences.email_on_incident_resolved,
            email_on_service_status_changed:
              preferences.email_on_service_status_changed,
            webhook_on_incident_created: preferences.webhook_on_incident_created,
            webhook_on_incident_updated: preferences.webhook_on_incident_updated,
            webhook_on_incident_resolved:
              preferences.webhook_on_incident_resolved,
            webhook_on_service_status_changed:
              preferences.webhook_on_service_status_changed,
            uptime_alert_threshold: preferences.uptime_alert_threshold ?? 95,
            uptime_alert_enabled: preferences.uptime_alert_enabled,
          }),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to save preferences");
      }

      const body = await res.json();
      setPreferences(body.data);
      toast.success("Notification preferences saved");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to save preferences",
      );
    } finally {
      setPreferencesLoading(false);
    }
  }

  async function handleCreateWebhook(e: React.FormEvent) {
    e.preventDefault();
    setWebhookLoading(true);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/webhooks`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(newWebhook),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to create webhook");
      }

      const body = await res.json();
      setWebhooks((current) => [body.data, ...current]);
      setNewWebhook({
        name: "",
        url: "",
        secret: "",
        event_types: ["incident.created", "incident.updated"],
      });
      toast.success("Webhook created");
      await refreshNotificationOperations();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to create webhook",
      );
    } finally {
      setWebhookLoading(false);
    }
  }

  async function toggleWebhook(webhook: WebhookConfig) {
    setWebhookLoading(true);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/webhooks/${webhook.id}`,
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ is_enabled: !webhook.is_enabled }),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to update webhook");
      }

      const body = await res.json();
      setWebhooks((current) =>
        current.map((item) => (item.id === webhook.id ? body.data : item)),
      );
      toast.success(
        body.data.is_enabled ? "Webhook enabled" : "Webhook disabled",
      );
      await refreshNotificationOperations();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to update webhook",
      );
    } finally {
      setWebhookLoading(false);
    }
  }

  async function deleteWebhook(webhook: WebhookConfig) {
    setWebhookLoading(true);

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/webhooks/${webhook.id}`,
        {
          method: "DELETE",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to delete webhook");
      }

      setWebhooks((current) => current.filter((item) => item.id !== webhook.id));
      toast.success("Webhook deleted");
      await refreshNotificationOperations();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to delete webhook",
      );
    } finally {
      setWebhookLoading(false);
    }
  }

  async function deleteSubscriber(subscriber: SubscriberListItem) {
    setSubscriberActionState({
      id: subscriber.id,
      action: "delete",
    });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/subscribers/${subscriber.id}`,
        {
          method: "DELETE",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to remove subscriber");
      }

      setSubscribers((current) =>
        current.filter((item) => item.id !== subscriber.id),
      );
      toast.success(
        subscriber.is_verified ? "Subscriber removed" : "Pending subscriber removed",
      );
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to remove subscriber",
      );
    } finally {
      setSubscriberActionState(null);
    }
  }

  async function resendSubscriberVerification(subscriber: SubscriberListItem) {
    setSubscriberActionState({
      id: subscriber.id,
      action: "resend",
    });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/subscribers/${subscriber.id}/resend`,
        {
          method: "POST",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(
          err.error?.message || "Failed to send another verification email",
        );
      }

      const body = await res.json();
      toast.success(body.data.message ?? "Queued another verification email");
      await refreshNotificationOperations();
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : "Failed to send another verification email",
      );
    } finally {
      setSubscriberActionState(null);
    }
  }

  async function retryEmailDelivery(entry: NotificationLogEntry) {
    setDeliveryActionState({ kind: "email", id: entry.id });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/deliveries/email/${entry.id}/retry`,
        {
          method: "POST",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to retry email delivery");
      }

      const body = await res.json();
      toast.success(body.data.message ?? "Queued another email delivery attempt");
      await refreshNotificationOperations();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to retry email delivery",
      );
    } finally {
      setDeliveryActionState(null);
    }
  }

  async function retryWebhookDelivery(entry: WebhookDeliveryEntry) {
    setDeliveryActionState({ kind: "webhook", id: entry.id });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/notifications/deliveries/webhooks/${entry.id}/retry`,
        {
          method: "POST",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to retry webhook delivery");
      }

      const body = await res.json();
      toast.success(
        body.data.message ?? "Queued another webhook delivery attempt",
      );
      await refreshNotificationOperations();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to retry webhook delivery",
      );
    } finally {
      setDeliveryActionState(null);
    }
  }

  function updatePreference(key: PreferenceKey, checked: boolean) {
    setPreferences((current) => ({
      ...current,
      [key]: checked,
    }));
  }

  function toggleWebhookEvent(eventType: string, checked: boolean) {
    setNewWebhook((current) => ({
      ...current,
      event_types: checked
        ? [...current.event_types, eventType]
        : current.event_types.filter((value) => value !== eventType),
    }));
  }

  async function verifyCustomDomain() {
    setDomainVerification((current) => ({
      ...current,
      loading: true,
      message: null,
    }));

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/custom-domain/verify`,
        {
          method: "POST",
        },
      );

      const body = await res.json();
      if (!res.ok) {
        throw new Error(body.error?.message || "Failed to verify custom domain");
      }

      setOrg((current) =>
        current
          ? {
              ...current,
              custom_domain_verified_at: body.data.is_ready
                ? new Date().toISOString()
                : current.custom_domain_verified_at,
            }
          : current,
      );
      setDomainVerification({
        loading: false,
        message: body.data.message,
        is_ready: body.data.is_ready,
        expected_target: body.data.expected_target,
      });
      toast.success(body.data.message);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to verify custom domain";
      setDomainVerification({
        loading: false,
        message,
        is_ready: false,
        expected_target: null,
      });
      toast.error(message);
    }
  }

  async function createMember(e: React.FormEvent) {
    e.preventDefault();
    setMemberLoading(true);

    try {
      const res = await fetch(`/api/proxy/api/organizations/${slug}/invitations`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: newMember.email.trim(),
          role: newMember.role,
        }),
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to create invitation");
      }

      const body = await res.json();
      setInvitations((current) => [body.data, ...current]);
      setNewMember({ email: "", role: "member" });
      toast.success("Invitation created");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to create invitation",
      );
    } finally {
      setMemberLoading(false);
    }
  }

  async function copyInvitationLink(invitation: Invitation) {
    setInvitationActionState({ id: invitation.id, action: "copy" });

    try {
      const link = `${window.location.origin}/invite/${invitation.token}`;
      await navigator.clipboard.writeText(link);
      toast.success("Invitation link copied");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to copy invitation link",
      );
    } finally {
      setInvitationActionState(null);
    }
  }

  async function deleteInvitation(invitation: Invitation) {
    setInvitationActionState({ id: invitation.id, action: "cancel" });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/invitations/${invitation.id}`,
        {
          method: "DELETE",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to cancel invitation");
      }

      setInvitations((current) =>
        current.filter((item) => item.id !== invitation.id),
      );
      toast.success("Invitation canceled");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to cancel invitation",
      );
    } finally {
      setInvitationActionState(null);
    }
  }

  async function resendInvitation(invitation: Invitation) {
    setInvitationActionState({ id: invitation.id, action: "resend" });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/invitations/${invitation.id}/resend`,
        {
          method: "POST",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to resend invitation");
      }

      const body = await res.json();
      setInvitations((current) =>
        current.map((item) => (item.id === invitation.id ? body.data : item)),
      );
      toast.success("Invitation email queued");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to resend invitation",
      );
    } finally {
      setInvitationActionState(null);
    }
  }

  async function updateMemberRole(member: MemberWithUser, role: MemberRole) {
    setMemberActionState({ id: member.id, action: "role" });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/members/${member.id}`,
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ role }),
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to update role");
      }

      const body = await res.json();
      setMembers((current) =>
        current.map((item) => (item.id === member.id ? body.data : item)),
      );
      toast.success("Member role updated");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to update role",
      );
    } finally {
      setMemberActionState(null);
    }
  }

  async function removeMember(member: MemberWithUser) {
    setMemberActionState({ id: member.id, action: "remove" });

    try {
      const res = await fetch(
        `/api/proxy/api/organizations/${slug}/members/${member.id}`,
        {
          method: "DELETE",
        },
      );

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message || "Failed to remove member");
      }

      setMembers((current) => current.filter((item) => item.id !== member.id));
      toast.success("Member removed");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to remove member",
      );
    } finally {
      setMemberActionState(null);
    }
  }

  if (!org) {
    return <div className="text-muted-foreground">{loadError ?? "Loading..."}</div>;
  }

  const entitlements = billingSummary?.entitlements ?? PLAN_FEATURES[org.plan];
  const customDomainLocked = !entitlements.custom_domain_enabled;
  const webhooksLocked = !entitlements.outbound_webhooks_enabled;
  const billingConfigured = Boolean(billingSummary?.billing_enabled);
  const upgradeOptions = billingSummary?.available_upgrades ?? [];
  const currentPlan = billingSummary?.current_plan ?? org.plan;
  const subscriptionStatus =
    billingSummary?.subscription_status ?? org.subscription_status;
  const downgradeState = billingSummary?.downgrade_state ?? org.downgrade_state;

  return (
    <div className="max-w-4xl space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      <Card>
        <CardHeader>
          <CardTitle>Plan & Usage</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">Current plan</span>
            <Badge variant="secondary" className="uppercase">
              {formatOrganizationPlan(org.plan)}
            </Badge>
          </div>
          <p className="text-sm text-muted-foreground">
            Monitors: {monitorCount ?? "..."} / {formatPlanMonitorLimit(org.plan)}
          </p>
          <div className="grid gap-2 text-sm text-muted-foreground sm:grid-cols-3">
            <div>
              Custom domains:{" "}
              {entitlements.custom_domain_enabled ? "Included" : "Upgrade required"}
            </div>
            <div>
              Outbound webhooks:{" "}
              {entitlements.outbound_webhooks_enabled
                ? "Included"
                : "Upgrade required"}
            </div>
            <div>
              Priority support:{" "}
              {entitlements.priority_support ? "Included" : "Not included"}
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Billing Status</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {downgradeState !== "none" ? (
            <div className="rounded-lg border border-amber-500/40 bg-amber-500/5 p-4 text-sm">
              <p className="font-medium">
                Downgrade state: {formatDowngradeState(downgradeState)}
              </p>
              <p className="mt-1 text-muted-foreground">
                {billingSummary?.downgrade_target_plan
                  ? `Target plan: ${formatOrganizationPlan(billingSummary.downgrade_target_plan)}.`
                  : "No lower-plan target is currently set."}{" "}
                {billingSummary?.downgrade_grace_ends_at
                  ? `Grace ends ${formatDateLabel(billingSummary.downgrade_grace_ends_at)}.`
                  : ""}
              </p>
              {billingSummary?.required_actions?.length ? (
                <div className="mt-3 space-y-1 text-muted-foreground">
                  {billingSummary.required_actions.map((action) => (
                    <p key={action}>- {action}</p>
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}
          <p className="text-sm text-muted-foreground">
            Managed billing supports self-serve upgrades, Stripe billing management,
            and a non-destructive downgrade grace period before lower-plan limits are enforced.
          </p>
          <div className="flex flex-wrap gap-2">
            <Badge variant="secondary" className="uppercase">
              {formatOrganizationPlan(currentPlan)}
            </Badge>
            <Badge variant={billingConfigured ? "secondary" : "outline"}>
              {billingConfigured ? "Billing configured" : "Billing not configured"}
            </Badge>
            <Badge
              variant={
                subscriptionStatus === "active" || subscriptionStatus === "trialing"
                  ? "secondary"
                  : "outline"
              }
            >
              {formatSubscriptionStatus(subscriptionStatus)}
            </Badge>
          </div>
          <div className="grid gap-2 text-sm text-muted-foreground sm:grid-cols-2">
            <div>
              Billing email: {billingSummary?.billing_email ?? "Not set yet"}
            </div>
            <div>
              Renewal / period end:{" "}
              {formatDateLabel(billingSummary?.current_period_end) ?? "Not available yet"}
            </div>
            <div>
              Customer record:{" "}
              {billingSummary?.stripe_customer_id ? "Connected" : "Not created yet"}
            </div>
            <div>
              Cancellation:{" "}
              {billingSummary?.cancel_at_period_end
                ? "Ends at the current period"
                : "No cancellation scheduled"}
            </div>
          </div>
          {billingSummary?.stripe_customer_id ? (
            <p className="text-xs text-muted-foreground">
              Stripe customer: {billingSummary.stripe_customer_id}
            </p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            {upgradeOptions.map((plan) => (
              <Button
                key={plan}
                type="button"
                disabled={!billingSummary?.checkout_enabled || billingActionState !== null}
                onClick={() => startCheckout(plan)}
              >
                {billingActionState === `checkout:${plan}`
                  ? "Redirecting..."
                  : `Upgrade to ${formatOrganizationPlan(plan)}`}
              </Button>
            ))}
            <Button
              type="button"
              variant="outline"
              disabled={!billingSummary?.portal_enabled || billingActionState !== null}
              onClick={openBillingPortal}
            >
              {billingActionState === "portal" ? "Opening..." : "Manage billing"}
            </Button>
          </div>
          {upgradeOptions.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {currentPlan === "team"
                ? "You are already on the highest managed beta plan."
                : "No higher plans are configured for this deployment yet."}
            </p>
          ) : null}
          <p className="text-xs text-muted-foreground">
            Portal access requires Stripe to be configured and this organization
            to have a customer record already.
          </p>
          {billingSummary?.entitlement_violations?.length ? (
            <div className="space-y-2 rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
              {billingSummary.entitlement_violations.map((violation) => (
                <p key={violation.code}>{violation.message}</p>
              ))}
            </div>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Organization</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSaveOrganization} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="slug">Slug</Label>
              <Input id="slug" value={org.slug} disabled />
              <p className="text-xs text-muted-foreground">
                Public page:{" "}
                {customDomain.trim()
                  ? `https://${customDomain.trim()}`
                  : `/s/${org.slug}`}
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="timezone">Timezone</Label>
              <Input
                id="timezone"
                value={timezone}
                onChange={(e) => setTimezone(e.target.value)}
                placeholder="UTC"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="logo-url">Logo URL</Label>
              <Input
                id="logo-url"
                value={logoUrl}
                onChange={(e) => setLogoUrl(e.target.value)}
                placeholder="https://example.com/logo.png"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="custom-domain">Custom Domain</Label>
              <Input
                id="custom-domain"
                value={customDomain}
                onChange={(e) => setCustomDomain(e.target.value)}
                placeholder="status.example.com"
                disabled={customDomainLocked}
              />
              <p className="text-xs text-muted-foreground">
                {customDomainLocked
                  ? "Custom domains unlock on Pro and Team plans. Upgrade to connect one."
                  : "Leave blank to keep using the default `/s/{org.slug}` URL."}
              </p>
              {!customDomainLocked && customDomain.trim() ? (
                <div className="flex flex-wrap items-center gap-2">
                  <Badge
                    variant={org.custom_domain_status === "verified" ? "secondary" : "outline"}
                  >
                    {org.custom_domain_status === "blocked_by_plan"
                      ? "Blocked by plan"
                      : org.custom_domain_status === "verified"
                        ? "Verified"
                        : "Verification pending"}
                  </Badge>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={verifyCustomDomain}
                    disabled={domainVerification.loading}
                  >
                    {domainVerification.loading ? "Verifying..." : "Verify Domain"}
                  </Button>
                  {domainVerification.expected_target ? (
                    <span className="text-xs text-muted-foreground">
                      Point DNS at {domainVerification.expected_target}
                    </span>
                  ) : null}
                </div>
              ) : null}
              {domainVerification.message ? (
                <p className="text-xs text-muted-foreground">
                  {domainVerification.message}
                </p>
              ) : null}
            </div>
            <div className="space-y-2">
              <Label htmlFor="color">Brand Color</Label>
              <div className="flex gap-2">
                <Input
                  type="color"
                  id="color"
                  value={brandColor}
                  onChange={(e) => setBrandColor(e.target.value)}
                  className="h-10 w-14 p-1"
                />
                <Input
                  value={brandColor}
                  onChange={(e) => setBrandColor(e.target.value)}
                  placeholder="#3B82F6"
                />
              </div>
            </div>
            <Button type="submit" disabled={loading}>
              {loading ? "Saving..." : "Save Changes"}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Team Members</CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          <p className="text-sm text-muted-foreground">
            Owners and admins can manage who has access to this organization.
            Invite teammates by email, then have them accept with the matching
            GitHub account.
          </p>

          <div className="space-y-4">
            {members.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No members found yet.
              </p>
            ) : (
              members.map((member) => (
                <div
                  key={member.id}
                  className="flex flex-col gap-3 rounded-lg border p-4 lg:flex-row lg:items-center lg:justify-between"
                >
                  <div className="space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-medium">
                        {member.user_name || member.user_email}
                      </p>
                      <Badge variant="outline" className="uppercase">
                        {member.role}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      {member.user_email}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <label className="flex items-center gap-2 text-sm text-muted-foreground">
                      <span>Role</span>
                      <select
                        aria-label={`Role for ${member.user_email}`}
                        className="h-9 rounded-md border bg-background px-3 text-sm text-foreground"
                        value={member.role}
                        disabled={memberActionState?.id === member.id}
                        onChange={(event) =>
                          updateMemberRole(
                            member,
                            event.target.value as MemberRole,
                          )
                        }
                      >
                        <option value="owner">Owner</option>
                        <option value="admin">Admin</option>
                        <option value="member">Member</option>
                      </select>
                    </label>
                    <Button
                      type="button"
                      variant="outline"
                      disabled={memberActionState?.id === member.id}
                      onClick={() => removeMember(member)}
                    >
                      {memberActionState?.id === member.id &&
                      memberActionState.action === "remove"
                        ? "Removing..."
                        : "Remove"}
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>

          <form onSubmit={createMember} className="space-y-4 rounded-lg border p-4">
            <h2 className="font-medium">Invite teammate</h2>
            <div className="space-y-2">
              <Label htmlFor="member-email">User email</Label>
              <Input
                id="member-email"
                type="email"
                value={newMember.email}
                onChange={(e) =>
                  setNewMember((current) => ({
                    ...current,
                    email: e.target.value,
                  }))
                }
                placeholder="teammate@example.com"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="member-role">Role</Label>
              <select
                id="member-role"
                className="h-10 w-full rounded-md border bg-background px-3 text-sm text-foreground"
                value={newMember.role}
                onChange={(e) =>
                  setNewMember((current) => ({
                    ...current,
                    role: e.target.value as MemberRole,
                  }))
                }
              >
                <option value="member">Member</option>
                <option value="admin">Admin</option>
                <option value="owner">Owner</option>
              </select>
            </div>
            <Button type="submit" disabled={memberLoading}>
              {memberLoading ? "Inviting..." : "Send Invite"}
            </Button>
          </form>

          <div className="space-y-4 rounded-lg border p-4">
            <h2 className="font-medium">Pending invitations</h2>
            {invitations.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No pending invitations right now.
              </p>
            ) : (
              invitations.map((invitation) => (
                <div
                  key={invitation.id}
                  className="flex flex-col gap-3 rounded-lg border p-4 lg:flex-row lg:items-center lg:justify-between"
                >
                  <div className="space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-medium">{invitation.email}</p>
                      <Badge variant="outline" className="uppercase">
                        {invitation.role}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Invited by {invitation.inviter_name || invitation.inviter_email} •
                      {" "}Expires {formatDateLabel(invitation.expires_at)}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Delivery: {invitation.delivery_status}
                      {invitation.last_sent_at
                        ? ` • Last sent ${formatDateLabel(invitation.last_sent_at)}`
                        : ""}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      disabled={
                        invitationActionState?.id === invitation.id ||
                        !["pending", "delivery_failed"].includes(
                          invitation.delivery_status,
                        )
                      }
                      onClick={() => resendInvitation(invitation)}
                    >
                      {invitationActionState?.id === invitation.id &&
                      invitationActionState.action === "resend"
                        ? "Resending..."
                        : "Resend Invite"}
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      disabled={invitationActionState?.id === invitation.id}
                      onClick={() => copyInvitationLink(invitation)}
                    >
                      {invitationActionState?.id === invitation.id &&
                      invitationActionState.action === "copy"
                        ? "Copying..."
                        : "Copy Invite Link"}
                    </Button>
                    <Button
                      type="button"
                      variant="destructive"
                      disabled={invitationActionState?.id === invitation.id}
                      onClick={() => deleteInvitation(invitation)}
                    >
                      {invitationActionState?.id === invitation.id &&
                      invitationActionState.action === "cancel"
                        ? "Canceling..."
                        : "Cancel Invite"}
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Notification Preferences</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            These defaults now control both subscriber email queueing and webhook
            queueing. Use the sections below to see who is subscribed and how
            recent deliveries have performed.
          </p>
          <form onSubmit={handleSavePreferences} className="space-y-6">
            <div className="space-y-3">
              <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                Email defaults
              </h2>
              <PreferenceToggle
                label="Email when an incident starts"
                checked={preferences.email_on_incident_created}
                onCheckedChange={(checked) =>
                  updatePreference("email_on_incident_created", checked)
                }
              />
              <PreferenceToggle
                label="Email when an incident is updated"
                checked={preferences.email_on_incident_updated}
                onCheckedChange={(checked) =>
                  updatePreference("email_on_incident_updated", checked)
                }
              />
              <PreferenceToggle
                label="Email when an incident is resolved"
                checked={preferences.email_on_incident_resolved}
                onCheckedChange={(checked) =>
                  updatePreference("email_on_incident_resolved", checked)
                }
              />
              <PreferenceToggle
                label="Email when service status changes"
                checked={preferences.email_on_service_status_changed}
                onCheckedChange={(checked) =>
                  updatePreference("email_on_service_status_changed", checked)
                }
              />
            </div>

            <div className="space-y-3">
              <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                Webhook defaults
              </h2>
              <PreferenceToggle
                label="Webhook when an incident starts"
                checked={preferences.webhook_on_incident_created}
                disabled={webhooksLocked}
                onCheckedChange={(checked) =>
                  updatePreference("webhook_on_incident_created", checked)
                }
              />
              <PreferenceToggle
                label="Webhook when an incident is updated"
                checked={preferences.webhook_on_incident_updated}
                disabled={webhooksLocked}
                onCheckedChange={(checked) =>
                  updatePreference("webhook_on_incident_updated", checked)
                }
              />
              <PreferenceToggle
                label="Webhook when an incident is resolved"
                checked={preferences.webhook_on_incident_resolved}
                disabled={webhooksLocked}
                onCheckedChange={(checked) =>
                  updatePreference("webhook_on_incident_resolved", checked)
                }
              />
              <PreferenceToggle
                label="Webhook when service status changes"
                checked={preferences.webhook_on_service_status_changed}
                disabled={webhooksLocked}
                onCheckedChange={(checked) =>
                  updatePreference("webhook_on_service_status_changed", checked)
                }
              />
              {webhooksLocked ? (
                <p className="text-xs text-muted-foreground">
                  Outbound webhooks unlock on Pro and Team plans.
                </p>
              ) : null}
            </div>

            <div className="space-y-3">
              <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                Uptime alerts
              </h2>
              <PreferenceToggle
                label="Enable uptime threshold alerts"
                checked={preferences.uptime_alert_enabled}
                onCheckedChange={(checked) =>
                  updatePreference("uptime_alert_enabled", checked)
                }
              />
              <div className="space-y-2">
                <Label htmlFor="uptime-threshold">Alert threshold (%)</Label>
                <Input
                  id="uptime-threshold"
                  type="number"
                  min="0"
                  max="100"
                  step="0.1"
                  value={preferences.uptime_alert_threshold ?? 95}
                  onChange={(e) =>
                    setPreferences((current) => ({
                      ...current,
                      uptime_alert_threshold: Number(e.target.value),
                    }))
                  }
                />
              </div>
            </div>

            <Button type="submit" disabled={preferencesLoading}>
              {preferencesLoading ? "Saving..." : "Save Notification Preferences"}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Subscribers</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Verified subscribers receive incident and service notifications based
            on the defaults above. Pending subscribers are still waiting for
            email confirmation.
          </p>
          {subscribers.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No subscribers yet. Public visitors can subscribe from the status
              page.
            </p>
          ) : (
            subscribers.map((subscriber) => (
              <div
                key={subscriber.id}
                className="flex flex-col gap-3 rounded-lg border p-4 sm:flex-row sm:items-center sm:justify-between"
              >
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <p className="font-medium">{subscriber.email}</p>
                    <Badge
                      variant={subscriber.is_verified ? "secondary" : "outline"}
                    >
                      {subscriber.is_verified ? "Verified" : "Pending"}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    {subscriber.is_verified
                      ? `Confirmed ${formatTimestamp(
                          subscriber.verified_at ?? subscriber.created_at,
                        )}`
                      : `Verification requested ${formatTimestamp(
                          subscriber.verification_sent_at ?? subscriber.created_at,
                        )}`}
                  </p>
                </div>
                <div className="flex flex-wrap gap-2">
                  {!subscriber.is_verified ? (
                    <Button
                      type="button"
                      variant="outline"
                      disabled={subscriberActionState?.id === subscriber.id}
                      onClick={() => resendSubscriberVerification(subscriber)}
                    >
                      {subscriberActionState?.id === subscriber.id &&
                      subscriberActionState.action === "resend"
                        ? "Resending..."
                        : "Resend Verification"}
                    </Button>
                  ) : null}
                  <Button
                    type="button"
                    variant="outline"
                    disabled={subscriberActionState?.id === subscriber.id}
                    onClick={() => deleteSubscriber(subscriber)}
                  >
                    {subscriberActionState?.id === subscriber.id &&
                    subscriberActionState.action === "delete"
                      ? "Removing..."
                      : subscriber.is_verified
                        ? "Remove Subscriber"
                        : "Remove Pending"}
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Recent Delivery Activity</CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-3">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <h2 className="font-medium">Email deliveries</h2>
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="outline">
                  {emailPagination.total} total
                </Badge>
                <DeliveryStatusFilter
                  label="Filter email deliveries"
                  value={emailStatusFilter}
                  onChange={(value) => {
                    setEmailStatusFilter(value);
                    setEmailPage(1);
                  }}
                />
                <PageSizeSelect
                  label="Email delivery page size"
                  value={deliveryPageSize}
                  onChange={(value) => {
                    setDeliveryPageSize(value);
                    setEmailPage(1);
                    setWebhookPage(1);
                  }}
                />
              </div>
            </div>
            {emailDeliveries.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No email deliveries yet. Once subscribers are verified and an
                event is triggered, results will show up here.
              </p>
            ) : (
              <div className="space-y-3">
                {emailDeliveries.map((entry) => (
                  <div key={entry.id} className="rounded-lg border p-4 space-y-2">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-medium">
                        {entry.subject ?? humanizeNotificationType(entry.notification_type)}
                      </p>
                      <StatusBadge status={entry.status} />
                    </div>
                    <p className="text-sm text-muted-foreground">
                      To {entry.recipient_email} • {humanizeNotificationType(entry.notification_type)}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Created {formatTimestamp(entry.created_at)}
                      {entry.sent_at ? ` • Sent ${formatTimestamp(entry.sent_at)}` : ""}
                      {entry.next_retry_at
                        ? ` • Next retry ${formatTimestamp(entry.next_retry_at)}`
                        : ""}
                      {entry.attempt_count > 0
                        ? ` • Attempt ${entry.attempt_count}/${entry.max_attempts}`
                        : ""}
                    </p>
                    {entry.error_message ? (
                      <p className="text-sm text-red-600">{entry.error_message}</p>
                    ) : null}
                    {entry.status === "failed" ? (
                      <div className="pt-1">
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          disabled={
                            deliveryActionState?.kind === "email" &&
                            deliveryActionState.id === entry.id
                          }
                          onClick={() => retryEmailDelivery(entry)}
                        >
                          {deliveryActionState?.kind === "email" &&
                          deliveryActionState.id === entry.id
                            ? "Retrying..."
                            : "Retry Delivery"}
                        </Button>
                      </div>
                    ) : null}
                  </div>
                ))}
                <DeliveryPaginationControls
                  page={emailPagination.page}
                  perPage={emailPagination.per_page}
                  total={emailPagination.total}
                  onPrevious={() =>
                    setEmailPage((current) => Math.max(1, current - 1))
                  }
                  onNext={() =>
                    setEmailPage((current) =>
                      current < totalPages(emailPagination) ? current + 1 : current,
                    )
                  }
                />
              </div>
            )}
          </div>

          <div className="space-y-3">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <h2 className="font-medium">Webhook deliveries</h2>
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="outline">
                  {webhookPagination.total} total
                </Badge>
                <DeliveryStatusFilter
                  label="Filter webhook deliveries"
                  value={webhookStatusFilter}
                  onChange={(value) => {
                    setWebhookStatusFilter(value);
                    setWebhookPage(1);
                  }}
                />
                <PageSizeSelect
                  label="Webhook delivery page size"
                  value={deliveryPageSize}
                  onChange={(value) => {
                    setDeliveryPageSize(value);
                    setEmailPage(1);
                    setWebhookPage(1);
                  }}
                />
              </div>
            </div>
            {webhookDeliveries.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No webhook deliveries yet. Create a webhook and trigger an event
                to see delivery outcomes here.
              </p>
            ) : (
              <div className="space-y-3">
                {webhookDeliveries.map((entry) => (
                  <div key={entry.id} className="rounded-lg border p-4 space-y-2">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-medium">{entry.webhook_name}</p>
                      <Badge variant="outline">{entry.event_type}</Badge>
                      <StatusBadge status={entry.status} />
                    </div>
                    <p className="text-sm text-muted-foreground">
                      {entry.webhook_url}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Created {formatTimestamp(entry.created_at)}
                      {entry.delivered_at
                        ? ` • Delivered ${formatTimestamp(entry.delivered_at)}`
                        : ""}
                      {entry.response_status_code
                        ? ` • HTTP ${entry.response_status_code}`
                        : ""}
                      {entry.next_retry_at
                        ? ` • Next retry ${formatTimestamp(entry.next_retry_at)}`
                        : ""}
                      {entry.attempt_count > 0
                        ? ` • Attempt ${entry.attempt_count}/${entry.max_attempts}`
                        : ""}
                    </p>
                    {entry.error_message ? (
                      <p className="text-sm text-red-600">{entry.error_message}</p>
                    ) : null}
                    {entry.status === "failed" ? (
                      <div className="pt-1">
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          disabled={
                            deliveryActionState?.kind === "webhook" &&
                            deliveryActionState.id === entry.id
                          }
                          onClick={() => retryWebhookDelivery(entry)}
                        >
                          {deliveryActionState?.kind === "webhook" &&
                          deliveryActionState.id === entry.id
                            ? "Retrying..."
                            : "Retry Delivery"}
                        </Button>
                      </div>
                    ) : null}
                  </div>
                ))}
                <DeliveryPaginationControls
                  page={webhookPagination.page}
                  perPage={webhookPagination.per_page}
                  total={webhookPagination.total}
                  onPrevious={() =>
                    setWebhookPage((current) => Math.max(1, current - 1))
                  }
                  onNext={() =>
                    setWebhookPage((current) =>
                      current < totalPages(webhookPagination)
                        ? current + 1
                        : current,
                    )
                  }
                />
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Webhooks</CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          <p className="text-sm text-muted-foreground">
            Configured webhook endpoints receive queued incident and service
            events with signed delivery and retry behavior. Recent outcomes now
            appear above so you can debug failures without leaving the dashboard.
          </p>
          {webhooksLocked ? (
            <p className="text-sm text-muted-foreground">
              Upgrade to Pro or Team to create or re-enable outbound webhooks.
            </p>
          ) : null}
          <div className="space-y-4">
            {webhooks.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No webhooks configured yet.
              </p>
            ) : (
              webhooks.map((webhook) => (
                <div key={webhook.id} className="rounded-lg border p-4 space-y-3">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="font-medium">{webhook.name}</p>
                      <p className="text-sm text-muted-foreground">
                        {webhook.url}
                      </p>
                    </div>
                    <Badge variant={webhook.is_enabled ? "secondary" : "outline"}>
                      {webhook.is_enabled ? "Enabled" : "Disabled"}
                    </Badge>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {webhook.event_types.map((eventType) => (
                      <Badge key={eventType} variant="outline">
                        {eventType}
                      </Badge>
                    ))}
                  </div>
                  <div className="flex gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      disabled={webhookLoading || (webhooksLocked && !webhook.is_enabled)}
                      onClick={() => toggleWebhook(webhook)}
                    >
                      {webhook.is_enabled ? "Disable" : "Enable"}
                    </Button>
                    <Button
                      type="button"
                      variant="destructive"
                      disabled={webhookLoading}
                      onClick={() => deleteWebhook(webhook)}
                    >
                      Delete
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>

          <form onSubmit={handleCreateWebhook} className="space-y-4 rounded-lg border p-4">
            <h2 className="font-medium">Add webhook</h2>
            <div className="space-y-2">
              <Label htmlFor="webhook-name">Name</Label>
              <Input
                id="webhook-name"
                value={newWebhook.name}
                onChange={(e) =>
                  setNewWebhook((current) => ({
                    ...current,
                    name: e.target.value,
                  }))
                }
                placeholder="Slack bridge"
                required
                disabled={webhooksLocked}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="webhook-url">URL</Label>
              <Input
                id="webhook-url"
                type="url"
                value={newWebhook.url}
                onChange={(e) =>
                  setNewWebhook((current) => ({
                    ...current,
                    url: e.target.value,
                  }))
                }
                placeholder="https://example.com/webhooks/statuspage"
                required
                disabled={webhooksLocked}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="webhook-secret">Signing secret</Label>
              <Input
                id="webhook-secret"
                value={newWebhook.secret}
                onChange={(e) =>
                  setNewWebhook((current) => ({
                    ...current,
                    secret: e.target.value,
                  }))
                }
                placeholder="At least 8 characters"
                required
                disabled={webhooksLocked}
              />
            </div>
            <div className="space-y-3">
              <Label>Events</Label>
              <div className="grid gap-2">
                {WEBHOOK_EVENT_OPTIONS.map((eventType) => (
                  <PreferenceToggle
                    key={eventType.value}
                    label={eventType.label}
                    checked={newWebhook.event_types.includes(eventType.value)}
                    disabled={webhooksLocked}
                    onCheckedChange={(checked) =>
                      toggleWebhookEvent(eventType.value, checked)
                    }
                  />
                ))}
              </div>
            </div>
            <Button type="submit" disabled={webhookLoading || webhooksLocked}>
              {webhookLoading
                ? "Saving..."
                : webhooksLocked
                  ? "Upgrade to unlock webhooks"
                  : "Create Webhook"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

function PreferenceToggle({
  label,
  checked,
  disabled = false,
  onCheckedChange,
}: {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <label
      className={`flex items-center justify-between gap-3 rounded-md border p-3 text-sm ${
        disabled ? "cursor-not-allowed opacity-60" : ""
      }`}
    >
      <span>{label}</span>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onCheckedChange(e.target.checked)}
        className="h-4 w-4"
      />
    </label>
  );
}

function formatDateLabel(value: string | null | undefined) {
  if (!value) {
    return null;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return null;
  }

  return parsed.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function DeliveryStatusFilter({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm text-muted-foreground">
      <span>{label}</span>
      <select
        aria-label={label}
        className="h-9 rounded-md border bg-background px-3 text-sm text-foreground"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        {DELIVERY_STATUS_OPTIONS.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function PageSizeSelect({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm text-muted-foreground">
      <span>{label}</span>
      <select
        aria-label={label}
        className="h-9 rounded-md border bg-background px-3 text-sm text-foreground"
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      >
        {[5, 10, 20].map((option) => (
          <option key={option} value={option}>
            {option} per page
          </option>
        ))}
      </select>
    </label>
  );
}

function DeliveryPaginationControls({
  page,
  perPage,
  total,
  onPrevious,
  onNext,
}: {
  page: number;
  perPage: number;
  total: number;
  onPrevious: () => void;
  onNext: () => void;
}) {
  const pages = totalPages({ page, per_page: perPage, total });

  return (
    <div className="flex flex-col gap-3 rounded-lg border p-4 text-sm sm:flex-row sm:items-center sm:justify-between">
      <p className="text-muted-foreground">
        Page {page} of {pages} • {total} total
      </p>
      <div className="flex gap-2">
        <Button type="button" variant="outline" disabled={page <= 1} onClick={onPrevious}>
          Previous
        </Button>
        <Button
          type="button"
          variant="outline"
          disabled={page >= pages}
          onClick={onNext}
        >
          Next
        </Button>
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const normalized = status.toLowerCase();
  if (normalized === "sent" || normalized === "success") {
    return <Badge variant="secondary">{humanizeStatus(status)}</Badge>;
  }
  if (normalized === "failed") {
    return <Badge variant="destructive">{humanizeStatus(status)}</Badge>;
  }
  return <Badge variant="outline">{humanizeStatus(status)}</Badge>;
}

function formatTimestamp(value: string | null) {
  if (!value) {
    return "unknown time";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function humanizeStatus(status: string) {
  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function humanizeNotificationType(type: string) {
  return type
    .replaceAll(".", " ")
    .replaceAll("_", " ")
    .split(" ")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function buildDeliveryQuery(page: number, perPage: number, status: string) {
  const params = new URLSearchParams({
    page: String(page),
    per_page: String(perPage),
  });

  if (status !== "all") {
    params.set("status", status);
  }

  return `?${params.toString()}`;
}

function totalPages(pagination: DeliveryPagination) {
  return Math.max(1, Math.ceil(pagination.total / pagination.per_page));
}
