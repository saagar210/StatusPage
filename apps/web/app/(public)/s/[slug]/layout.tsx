export default function PublicStatusLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen bg-background">
      {children}
      <footer className="border-t py-6 text-center text-sm text-muted-foreground">
        Powered by{" "}
        <a href="/" className="underline hover:text-foreground">
          StatusPage.sh
        </a>
      </footer>
    </div>
  );
}
