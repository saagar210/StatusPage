import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Activity, Bell, Globe, Server } from "lucide-react";

export default function LandingPage() {
  return (
    <div className="flex flex-col items-center">
      {/* Hero */}
      <section className="w-full py-24 md:py-32">
        <div className="container mx-auto px-4 text-center">
          <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
            Open-source status pages
            <br />
            that just work
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            Monitor your services, communicate incidents, and keep your users
            informed. Self-host for free or use our managed platform.
          </p>
          <div className="mt-10 flex items-center justify-center gap-4">
            <Button asChild size="lg">
              <Link href="/login">Get Started</Link>
            </Button>
            <Button variant="outline" size="lg" asChild>
              <a
                href="https://github.com/statuspage-sh/statuspage"
                target="_blank"
                rel="noopener noreferrer"
              >
                Star on GitHub
              </a>
            </Button>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="w-full border-t py-24">
        <div className="container mx-auto px-4">
          <h2 className="text-center text-3xl font-bold">
            Everything you need
          </h2>
          <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader>
                <Activity className="h-8 w-8 mb-2 text-primary" />
                <CardTitle>Uptime Monitoring</CardTitle>
                <CardDescription>
                  HTTP, TCP, DNS, and ICMP checks at configurable intervals.
                  Automatic incident creation on failure.
                </CardDescription>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader>
                <Bell className="h-8 w-8 mb-2 text-primary" />
                <CardTitle>Incident Management</CardTitle>
                <CardDescription>
                  Create and manage incidents with timeline updates. Keep your
                  users in the loop with real-time status changes.
                </CardDescription>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader>
                <Globe className="h-8 w-8 mb-2 text-primary" />
                <CardTitle>Public Status Page</CardTitle>
                <CardDescription>
                  Beautiful, branded status pages with 90-day uptime history.
                  Custom domains supported.
                </CardDescription>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader>
                <Server className="h-8 w-8 mb-2 text-primary" />
                <CardTitle>Self-Hostable</CardTitle>
                <CardDescription>
                  MIT licensed. Deploy with Docker Compose in minutes. Your
                  data, your infrastructure.
                </CardDescription>
              </CardHeader>
            </Card>
          </div>
        </div>
      </section>
    </div>
  );
}
