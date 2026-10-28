import type { ReactNode } from "react";
import { Download, RotateCcw, X } from "lucide-react";
import { IconButton } from "./icon-button";
import { Button } from "./ui/button";
import { useApp } from "@/app/context";
import { useTask } from "@/hooks/use-task";
import { download, openFile } from "@/transfers/download";
import type { PublicShare, Share } from "@/types/api";

export function FileOpenButton({
  share,
  resource,
  management = false,
  password,
  onStarted,
  children,
}: {
  share: Share | PublicShare;
  resource: string;
  management?: boolean;
  password?: string | null;
  onStarted?: (resource: string, href: string) => void;
  children: string;
}) {
  const { csrf } = useApp();
  const { pending, run } = useTask();
  return (
    <Button
      type="button"
      variant="link"
      className="file-open-link h-auto w-full min-w-0 whitespace-normal p-0 text-left text-[13px]"
      title={`${children} · Open in browser`}
      aria-label={`Open ${children} in browser (new tab)`}
      aria-busy={pending}
      disabled={pending}
      onClick={() => {
        void run(async (signal) => {
          const href = await openFile({
            shareId: share.id,
            resource,
            management,
            scope: management ? csrf : "public",
            password,
            signal,
          });
          onStarted?.(resource, href);
        });
      }}
    >
      <strong>{children}</strong>
    </Button>
  );
}

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
      {Object.keys(links).length > 0 && (
        <p className="fine muted retry-guidance">
          Download started. Retry an interrupted transfer below.
        </p>
      )}
      {Object.entries(links).map(([resource, href]) => {
        const name =
          resource === "zip"
            ? "All files (ZIP)"
            : share.files.find((f) => f.id === resource)?.name;
        return (
          <div
            className="transfer-feedback"
            data-resource={resource}
            role="status"
            key={resource}
          >
            <span className="feedback-file" title={name}>
              {name}
            </span>
            <a
              href={href}
              data-action="public-download"
              data-resource={resource}
              onClick={(e) => {
                e.preventDefault();
                retry(resource);
              }}
              aria-label="Retry this download"
              title={`Retry ${name}`}
            >
              <RotateCcw aria-hidden="true" className="inline-icon" />
              Retry
            </a>
            <IconButton
              size="icon-xs"
              label="Dismiss download message"
              onClick={() => dismiss(resource)}
            >
              <X aria-hidden="true" />
            </IconButton>
          </div>
        );
      })}
    </div>
  );
}
