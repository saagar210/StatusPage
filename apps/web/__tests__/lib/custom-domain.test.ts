import { describe, expect, it } from "vitest";
import { buildPublicBasePath, buildPublicHref } from "@/lib/custom-domain";

describe("custom-domain helpers", () => {
  it("uses slug paths when no custom domain is resolved", () => {
    expect(buildPublicBasePath("demo", null)).toBe("/s/demo");
    expect(buildPublicHref("/s/demo", "/history")).toBe("/s/demo/history");
  });

  it("uses root paths when a custom domain is resolved", () => {
    const resolved = {
      slug: "demo",
      organization: {
        name: "Demo",
        logo_url: null,
        brand_color: "#2563eb",
      },
    };

    expect(buildPublicBasePath("demo", resolved)).toBe("");
    expect(buildPublicHref("", "/history")).toBe("/history");
    expect(buildPublicHref("", "")).toBe("/");
  });
});
