import { NextRequest } from "next/server";
import { auth } from "@/lib/auth";

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

  if (!session?.user) {
    return new Response("Unauthorized", { status: 401 });
  }

  const searchParams = request.nextUrl.searchParams;
  const orgId = searchParams.get("org_id");

  if (!orgId) {
    return new Response("Missing org_id parameter", { status: 400 });
  }

  // Create SSE stream
  const stream = new ReadableStream({
    start(controller) {
      const encoder = new TextEncoder();

      // Send initial connection message
      const sendEvent = (event: string, data: any) => {
        const message = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
        controller.enqueue(encoder.encode(message));
      };

      sendEvent("connected", {
        message: "Connected to real-time updates",
        org_id: orgId,
        timestamp: new Date().toISOString(),
      });

      // Keep connection alive with periodic heartbeats
      const heartbeatInterval = setInterval(() => {
        try {
          sendEvent("heartbeat", { timestamp: new Date().toISOString() });
        } catch (error) {
          clearInterval(heartbeatInterval);
          controller.close();
        }
      }, 30000); // 30 seconds

      // TODO: Subscribe to Redis pub/sub channels
      // For now, this is a placeholder that can be extended when Redis integration is complete
      // In production, you'd:
      // 1. Connect to Redis
      // 2. Subscribe to org:{orgId}:service:status, org:{orgId}:incident:created, etc.
      // 3. Forward Redis messages to SSE stream
      // 4. Handle cleanup on disconnect

      // Handle client disconnect
      request.signal.addEventListener("abort", () => {
        clearInterval(heartbeatInterval);
        controller.close();
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
