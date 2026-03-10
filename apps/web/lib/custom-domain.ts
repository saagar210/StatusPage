import { headers } from "next/headers";
import { cache } from "react";
import { resolveCustomDomain } from "@/lib/api-client";
import type { ResolvedCustomDomain } from "@/lib/types";

function normalizeHostHeader(rawHost: string | null): string | null {
  if (!rawHost) {
    return null;
  }

  const firstValue = rawHost
    .split(",")
    .map((value) => value.trim())
    .find(Boolean);

  if (!firstValue) {
    return null;
  }

  const host = firstValue.replace(/\.$/, "").toLowerCase();
  if (host.startsWith("[") && host.endsWith("]")) {
    return host;
  }

  const lastColon = host.lastIndexOf(":");
  if (lastColon > -1) {
    const maybePort = host.slice(lastColon + 1);
    const maybeHost = host.slice(0, lastColon);
    if (/^\d+$/.test(maybePort) && maybeHost) {
      return maybeHost;
    }
  }

  return host;
}

function isLocalHost(host: string): boolean {
  return (
    host === "localhost" ||
    host === "127.0.0.1" ||
    host === "0.0.0.0" ||
    host.endsWith(".localhost")
  );
}

export const resolveCustomDomainFromHeaders = cache(async () => {
  const headerStore = await headers();
  const host = normalizeHostHeader(
    headerStore.get("x-forwarded-host") ?? headerStore.get("host"),
  );

  if (!host || isLocalHost(host)) {
    return null;
  }

  try {
    return await resolveCustomDomain(host);
  } catch {
    return null;
  }
});

export function buildPublicBasePath(
  slug: string,
  resolvedCustomDomain: ResolvedCustomDomain | null,
): string {
  return resolvedCustomDomain ? "" : `/s/${slug}`;
}

export function buildPublicHref(basePath: string, path = ""): string {
  if (!path) {
    return basePath || "/";
  }

  if (!basePath) {
    return path.startsWith("/") ? path : `/${path}`;
  }

  return `${basePath}${path.startsWith("/") ? path : `/${path}`}`;
}
