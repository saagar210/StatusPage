import Link from "next/link";
import { resolveCustomDomainFromHeaders } from "@/lib/custom-domain";

export default async function MarketingLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const resolvedCustomDomain = await resolveCustomDomainFromHeaders();

  if (resolvedCustomDomain) {
    return (
      <div className="min-h-screen bg-background">
        {children}
        <footer className="border-t py-6 text-center text-sm text-muted-foreground">
          Powered by{" "}
          <Link href="/" className="underline hover:text-foreground">
            StatusPage.sh
          </Link>
        </footer>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col">
      <header className="border-b">
        <div className="container mx-auto flex h-16 items-center justify-between px-4">
          <Link href="/" className="text-xl font-bold">
            StatusPage.sh
          </Link>
          <nav className="flex items-center gap-6">
            <Link
              href="/login"
              className="text-sm font-medium text-muted-foreground hover:text-foreground"
            >
              Sign In
            </Link>
          </nav>
        </div>
      </header>
      <main className="flex-1">{children}</main>
      <footer className="border-t py-8">
        <div className="container mx-auto px-4 text-center text-sm text-muted-foreground">
          StatusPage.sh - Open source status pages
        </div>
      </footer>
    </div>
  );
}
