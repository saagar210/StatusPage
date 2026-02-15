"use client";

import { useEffect, useState, useRef, useCallback } from "react";

type ServiceStatus = "operational" | "degraded" | "offline" | "maintenance";

interface ServiceStatusEvent {
  service_id: string;
  service_name: string;
  old_status: ServiceStatus;
  new_status: ServiceStatus;
  timestamp: string;
}

interface IncidentCreatedEvent {
  incident_id: string;
  title: string;
  status: string;
  impact: string;
  affected_services: string[];
  timestamp: string;
}

interface IncidentUpdatedEvent {
  incident_id: string;
  update_id: string;
  status: string;
  message: string;
  timestamp: string;
}

/**
 * Hook to subscribe to real-time service status updates for an organization
 *
 * @param orgId - Organization ID to subscribe to
 * @param onServiceStatusChange - Callback when service status changes
 * @param enabled - Whether the subscription is active (default: true)
 *
 * @example
 * ```tsx
 * const { connected, error } = useRealtimeStatus(orgId, (event) => {
 *   console.log(`Service ${event.service_name} is now ${event.new_status}`);
 *   // Update local state, refetch data, show toast, etc.
 * });
 * ```
 */
export function useRealtimeStatus(
  orgId: string | undefined,
  onServiceStatusChange: (event: ServiceStatusEvent) => void,
  enabled: boolean = true
) {
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!orgId || !enabled) {
      return;
    }

    // Create EventSource connection to SSE endpoint
    const eventSource = new EventSource(`/api/realtime?org_id=${orgId}`);
    eventSourceRef.current = eventSource;

    eventSource.addEventListener("connected", () => {
      setConnected(true);
      setError(null);
      console.log("[Realtime] Connected to status updates");
    });

    eventSource.addEventListener("service:status", (event) => {
      try {
        const data: ServiceStatusEvent = JSON.parse(event.data);
        onServiceStatusChange(data);
      } catch (err) {
        console.error("[Realtime] Failed to parse service status event:", err);
      }
    });

    eventSource.addEventListener("heartbeat", () => {
      // Connection is alive
    });

    eventSource.onerror = () => {
      setConnected(false);
      setError("Connection lost. Retrying...");
      console.error("[Realtime] Connection error");
    };

    // Cleanup on unmount
    return () => {
      eventSource.close();
      setConnected(false);
    };
  }, [orgId, enabled, onServiceStatusChange]);

  return { connected, error };
}

/**
 * Hook to subscribe to real-time incident updates for an organization
 *
 * @param orgId - Organization ID to subscribe to
 * @param onIncidentCreated - Callback when new incident is created
 * @param onIncidentUpdated - Callback when incident is updated
 * @param enabled - Whether the subscription is active (default: true)
 *
 * @example
 * ```tsx
 * const { connected } = useRealtimeIncidents(
 *   orgId,
 *   (event) => {
 *     toast.error(`New incident: ${event.title}`);
 *     queryClient.invalidateQueries(['incidents']);
 *   },
 *   (event) => {
 *     toast.info(`Incident updated: ${event.message}`);
 *     queryClient.invalidateQueries(['incidents', event.incident_id]);
 *   }
 * );
 * ```
 */
export function useRealtimeIncidents(
  orgId: string | undefined,
  onIncidentCreated: (event: IncidentCreatedEvent) => void,
  onIncidentUpdated: (event: IncidentUpdatedEvent) => void,
  enabled: boolean = true
) {
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!orgId || !enabled) {
      return;
    }

    const eventSource = new EventSource(`/api/realtime?org_id=${orgId}`);
    eventSourceRef.current = eventSource;

    eventSource.addEventListener("connected", () => {
      setConnected(true);
      setError(null);
    });

    eventSource.addEventListener("incident:created", (event) => {
      try {
        const data: IncidentCreatedEvent = JSON.parse(event.data);
        onIncidentCreated(data);
      } catch (err) {
        console.error("[Realtime] Failed to parse incident created event:", err);
      }
    });

    eventSource.addEventListener("incident:updated", (event) => {
      try {
        const data: IncidentUpdatedEvent = JSON.parse(event.data);
        onIncidentUpdated(data);
      } catch (err) {
        console.error("[Realtime] Failed to parse incident updated event:", err);
      }
    });

    eventSource.onerror = () => {
      setConnected(false);
      setError("Connection lost. Retrying...");
    };

    return () => {
      eventSource.close();
      setConnected(false);
    };
  }, [orgId, enabled, onIncidentCreated, onIncidentUpdated]);

  return { connected, error };
}

/**
 * Generic hook for real-time organization events
 *
 * @param orgId - Organization ID
 * @param eventHandlers - Map of event types to handlers
 * @param enabled - Whether subscription is active
 *
 * @example
 * ```tsx
 * useRealtimeOrg(orgId, {
 *   'service:status': (data) => console.log('Service status changed', data),
 *   'incident:created': (data) => console.log('New incident', data),
 *   'monitor:check': (data) => console.log('Monitor check result', data),
 * });
 * ```
 */
export function useRealtimeOrg(
  orgId: string | undefined,
  eventHandlers: Record<string, (data: any) => void>,
  enabled: boolean = true
) {
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  // Memoize event handlers to prevent unnecessary re-subscriptions
  const handlersRef = useRef(eventHandlers);
  useEffect(() => {
    handlersRef.current = eventHandlers;
  }, [eventHandlers]);

  useEffect(() => {
    if (!orgId || !enabled) {
      return;
    }

    const eventSource = new EventSource(`/api/realtime?org_id=${orgId}`);
    eventSourceRef.current = eventSource;

    eventSource.addEventListener("connected", () => {
      setConnected(true);
      setError(null);
    });

    // Register all event handlers
    Object.keys(handlersRef.current).forEach((eventType) => {
      eventSource.addEventListener(eventType, (event) => {
        try {
          const data = JSON.parse(event.data);
          handlersRef.current[eventType]?.(data);
        } catch (err) {
          console.error(`[Realtime] Failed to parse ${eventType} event:`, err);
        }
      });
    });

    eventSource.onerror = () => {
      setConnected(false);
      setError("Connection lost. Retrying...");
    };

    return () => {
      eventSource.close();
      setConnected(false);
    };
  }, [orgId, enabled]);

  const close = useCallback(() => {
    eventSourceRef.current?.close();
    setConnected(false);
  }, []);

  return { connected, error, close };
}
