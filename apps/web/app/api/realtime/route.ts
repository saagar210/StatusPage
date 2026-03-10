import { NextRequest } from "next/server";
import { createClient } from "redis";
import { auth, pool } from "@/lib/auth";

type RealtimePayload = Record<string, unknown>;

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

/**
 * Real-time Server-Sent Events (SSE) endpoint
 *
 * Subscribes to Redis pub/sub channels for the authenticated user's organization
 * and streams events to the client.
 *
 * Usage from client:
 *   const eventSource = new EventSource('/api/realtime?org_id=...');
 *   eventSource.addEventListener('service:status', (event) => {
 *     const data = JSON.parse(event.data);
 *     // Update UI
 *   });
 */
export async function GET(request: NextRequest) {
  const session = await auth();
  const userId = session?.user?.id;

  if (!userId) {
    return new Response("Unauthorized", { status: 401 });
  }

  const searchParams = request.nextUrl.searchParams;
  const slug = searchParams.get("slug");

  if (!slug) {
    return new Response("Missing slug parameter", { status: 400 });
  }

  const orgResult = await pool.query<{
    id: string;
    slug: string;
  }>(
    `
      SELECT o.id, o.slug
      FROM organizations o
      JOIN members m ON m.org_id = o.id
      WHERE o.slug = $1 AND m.user_id = $2
      LIMIT 1
    `,
    [slug, userId],
  );

  const organization = orgResult.rows[0];

  if (!organization) {
    return new Response("Organization not found", { status: 404 });
  }

  // Create SSE stream
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const redisClient = createClient({
        url: process.env.REDIS_URL || "redis://localhost:6379",
      });

      let closed = false;
      const subscriptions = new Map<string, string>([
        [`org:${organization.id}:service:status`, "service:status"],
        [`org:${organization.id}:incident:created`, "incident:created"],
        [`org:${organization.id}:incident:updated`, "incident:updated"],
      ]);

      const close = async () => {
        if (closed) return;
        closed = true;
        clearInterval(heartbeatInterval);

        try {
          for (const channel of subscriptions.keys()) {
            await redisClient.unsubscribe(channel);
          }
        } catch {
          // Ignore unsubscribe failures during shutdown.
        }

        try {
          if (redisClient.isOpen) {
            await redisClient.quit();
          }
        } catch {
          // Ignore quit failures during shutdown.
        }

        try {
          controller.close();
        } catch {
          // Ignore double-close races.
        }
      };

      // Send initial connection message
      const sendEvent = (event: string, data: RealtimePayload) => {
        if (closed) return false;
        const message = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
        try {
          controller.enqueue(encoder.encode(message));
          return true;
        } catch {
          void close();
          return false;
        }
      };

      sendEvent("connected", {
        message: "Connected to real-time updates",
        org_id: organization.id,
        slug: organization.slug,
        timestamp: new Date().toISOString(),
      });

      // Keep connection alive with periodic heartbeats
      const heartbeatInterval = setInterval(() => {
        try {
          sendEvent("heartbeat", { timestamp: new Date().toISOString() });
        } catch {
          void close();
        }
      }, 30000); // 30 seconds

      redisClient.on("error", (error) => {
        console.error("[Realtime] Redis SSE bridge error:", error);
        try {
          sendEvent("error", {
            message: "Realtime connection error",
            timestamp: new Date().toISOString(),
          });
        } catch {
          // Ignore stream write failures during error handling.
        }
      });

      try {
        await redisClient.connect();

        for (const [channel, eventName] of subscriptions.entries()) {
          await redisClient.subscribe(channel, (message) => {
            try {
              if (closed) return;
              const parsed = JSON.parse(message) as RealtimePayload;
              sendEvent(eventName, parsed);
            } catch (error) {
              console.error(
                `[Realtime] Failed to parse ${eventName} payload:`,
                error,
              );
            }
          });
        }
      } catch (error) {
        console.error("[Realtime] Failed to establish Redis subscription:", error);
        await close();
        controller.error(error);
        return;
      }

      // Handle client disconnect
      request.signal.addEventListener("abort", () => {
        void close();
      });
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
    },
  });
}
