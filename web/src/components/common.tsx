import type { ComponentProps, ReactNode } from "react";
import { Card, CardHeader, CardTitle, CardContent } from "./ui/card";
import { cn } from "@/lib/utils";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { Alert, AlertTitle, AlertDescription } from "./ui/alert";
import { errorMessage } from "@/lib/api";

export function Heading({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children?: ReactNode;
}) {
  return (
    <div className="page-heading">
      <div>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {children}
    </div>
  );
}
export const statusLabels: Record<string, string> = {
  active: "Available",
  draft: "Draft",
  preparing: "Preparing",
  expired: "Expired",
  exhausted: "Allowance used",
  revoked: "Removed",
  abandoned: "Abandoned",
};
export function Status({ state }: { state: string }) {
  return (
    <Badge
      variant={state === "active" ? "default" : "secondary"}
      className={`badge ${state}`}
    >
      {statusLabels[state] || state}
    </Badge>
  );
}
export function Loading({ error }: { error?: unknown }) {
  return error ? (
    <Alert variant="destructive">
      <AlertDescription className="min-w-0 [overflow-wrap:anywhere]">
        <p>{errorMessage(error)}</p>
      </AlertDescription>
    </Alert>
  ) : (
    <p className="muted" role="status">
      Opening Binbin…
    </p>
  );
}

export function Panel({
  title,
  className,
  children,
  ...props
}: Omit<ComponentProps<typeof Card>, "title"> & { title?: ReactNode }) {
  return (
    <Card className={cn("min-w-0", className)} {...props}>
      {title && (
        <CardHeader>
          <CardTitle>
            <h2>{title}</h2>
          </CardTitle>
        </CardHeader>
      )}
      <CardContent className="min-w-0">{children}</CardContent>
    </Card>
  );
}

export function RefreshError({
  error,
  retry,
}: {
  error?: unknown;
  retry: () => void;
}) {
  if (!error) return null;
  return (
    <Alert variant="destructive" className="mb-6 min-w-0">
      <AlertTitle>
        Couldn’t refresh. Details shown may be out of date.
      </AlertTitle>
      <AlertDescription className="min-w-0 [overflow-wrap:anywhere]">
        {errorMessage(error)}
      </AlertDescription>
      <Button variant="outline" className="mt-2 w-fit" onClick={retry}>
        Try again
      </Button>
    </Alert>
  );
}
