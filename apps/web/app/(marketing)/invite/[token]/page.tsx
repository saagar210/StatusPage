import { auth, signIn } from "@/lib/auth";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { AcceptInvitationCard } from "@/components/invitations/accept-invitation-card";

export default async function InvitationPage({
  params,
}: {
  params: Promise<{ token: string }>;
}) {
  const { token } = await params;
  const session = await auth();

  if (!session) {
    return (
      <div className="flex min-h-[60vh] items-center justify-center">
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <CardTitle>Sign in to accept your invitation</CardTitle>
            <CardDescription>
              Use the GitHub account that matches the invited email address.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form
              action={async () => {
                "use server";
                await signIn("github", { redirectTo: `/invite/${token}` });
              }}
            >
              <Button className="w-full" type="submit">
                Sign in with GitHub
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex min-h-[60vh] items-center justify-center">
      <AcceptInvitationCard token={token} />
    </div>
  );
}
