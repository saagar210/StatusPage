"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";

export function AcceptInvitationCard({ token }: { token: string }) {
  const router = useRouter();
  const [loading, setLoading] = useState(false);

  async function acceptInvitation() {
    setLoading(true);

    try {
      const res = await fetch(`/api/proxy/api/invitations/${token}/accept`, {
        method: "POST",
      });
      const body = await res.json();

      if (!res.ok) {
        throw new Error(body.error?.message || "Failed to accept invitation");
      }

      toast.success(body.data.message);
      router.push(`/dashboard/${body.data.org_slug}`);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to accept invitation",
      );
    } finally {
      setLoading(false);
    }
  }

  return (
    <Card className="w-full max-w-md">
      <CardHeader className="text-center">
        <CardTitle>Accept invitation</CardTitle>
        <CardDescription>
          Join the invited organization with your current GitHub account.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Button className="w-full" onClick={acceptInvitation} disabled={loading}>
          {loading ? "Accepting..." : "Accept Invitation"}
        </Button>
      </CardContent>
    </Card>
  );
}
