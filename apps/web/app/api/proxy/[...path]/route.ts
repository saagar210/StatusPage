import { cookies } from "next/headers";
import { NextRequest, NextResponse } from "next/server";

const INTERNAL_API_URL =
  process.env.INTERNAL_API_URL || "http://localhost:4000";

const ALLOWED_PREFIXES = ["/api/organizations", "/api/public"];

async function proxyRequest(req: NextRequest) {
  const url = new URL(req.url);
  const path = url.pathname.replace("/api/proxy", "");

  // Security: only allow specific API paths
  if (!ALLOWED_PREFIXES.some((prefix) => path.startsWith(prefix))) {
    return NextResponse.json(
      { error: { code: "FORBIDDEN", message: "Path not allowed" } },
      { status: 403 },
    );
  }

  const cookieStore = await cookies();
  const sessionToken =
    cookieStore.get("authjs.session-token")?.value ||
    cookieStore.get("__Secure-authjs.session-token")?.value;

  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };
  if (sessionToken) {
    headers["Cookie"] = `authjs.session-token=${sessionToken}`;
  }

  const targetUrl = `${INTERNAL_API_URL}${path}${url.search}`;

  const fetchOptions: RequestInit = {
    method: req.method,
    headers,
  };

  if (req.method !== "GET" && req.method !== "HEAD") {
    const body = await req.text();
    if (body) {
      fetchOptions.body = body;
    }
  }

  const res = await fetch(targetUrl, fetchOptions);

  const responseBody = await res.text();
  return new NextResponse(responseBody || null, {
    status: res.status,
    headers: { "Content-Type": "application/json" },
  });
}

export const GET = proxyRequest;
export const POST = proxyRequest;
export const PATCH = proxyRequest;
export const DELETE = proxyRequest;
export const PUT = proxyRequest;
