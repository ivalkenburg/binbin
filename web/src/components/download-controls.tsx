import type { ReactNode } from "react";
import { Download, RotateCcw, X } from "lucide-react";
import { IconButton } from "./icon-button";
import { Button } from "./ui/button";
import { Alert, AlertTitle, AlertDescription, AlertAction } from "./ui/alert";
import { useApp } from "@/app/context";
import { useTask } from "@/hooks/use-task";
import { download } from "@/transfers/download";
import type { PublicShare, Share } from "@/types/api";

export function DownloadButton({
  share,
  resource,
  management = false,
  password,
  onStarted,
  children,
  primary = false,
  className,
}: {
  share: Share | PublicShare;
  resource: string;
  management?: boolean;
  password?: string | null;
  onStarted?: (resource: string, href: string) => void;
  children?: ReactNode;
  primary?: boolean;
  className?: string;
}) {
  const { csrf, notify } = useApp();
  const { pending, run } = useTask();
  return (
    <Button
      type="button"
      variant={primary ? "default" : "outline"}
      className={className}
      aria-busy={pending}
      data-action={management ? "download" : "public-download"}
      data-resource={resource}
      onClick={() =>
        void run(async (signal) => {
          const href = await download({
            shareId: share.id,
            resource,
            management,
            scope: management ? csrf : "public",
            password,
            signal,
          });
          if (management)
            notify("Download started. Your allowance is unchanged.");
          else {
            notify("");
            onStarted?.(resource, href);
          }
        })
      }
    >
      <Download aria-hidden="true" />
      {children || "Download"}
    </Button>
  );
}
export function DownloadFeedback({
  share,
  links,
  dismiss,
  retry,
}: {
  share: PublicShare;
  links: Record<string, string>;
  dismiss: (resource: string) => void;
  retry: (resource: string) => void;
}) {
  return (
    <div id="retry-links" aria-live="polite">
      {Object.entries(links).map(([resource, href]) => {
        const name =
          resource === "zip"
            ? "All files (ZIP)"
            : share.files.find((f) => f.id === resource)?.name;
        return (
          <Alert
            className="transfer-feedback"
            data-resource={resource}
            role="status"
            key={resource}
          >
            <div className="min-w-0">
              <AlertTitle>Download started</AlertTitle>
              <span className="feedback-file" title={name}>
                {name}
              </span>
              <AlertDescription className="my-2 text-xs">
                Connection interrupted? Resume with the same link.
              </AlertDescription>
              <a
                href={href}
                data-action="public-download"
                data-resource={resource}
                onClick={(e) => {
                  e.preventDefault();
                  retry(resource);
                }}
              >
                <RotateCcw aria-hidden="true" className="inline-icon" />
                Retry this download
              </a>
            </div>
            <AlertAction>
              <IconButton
                size="icon-xs"
                label="Dismiss download message"
                onClick={() => dismiss(resource)}
              >
                <X aria-hidden="true" />
              </IconButton>
            </AlertAction>
          </Alert>
        );
      })}
    </div>
  );
}
