import type { ReactNode } from "react";
import { Download } from "lucide-react";
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
  children,
}: {
  share: Share | PublicShare;
  resource: string;
  management?: boolean;
  password?: string | null;
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
          await openFile({
            shareId: share.id,
            resource,
            management,
            scope: management ? csrf : "public",
            password,
            signal,
          });
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
  children,
  primary = false,
  iconOnly = false,
  title,
  className,
  onPrepared,
}: {
  share: Share | PublicShare;
  resource: string;
  management?: boolean;
  password?: string | null;
  children?: ReactNode;
  primary?: boolean;
  iconOnly?: boolean;
  title?: string;
  className?: string;
  onPrepared?: (href: string) => Promise<void>;
}) {
  const { csrf, notify } = useApp();
  const { pending, run } = useTask();
  return (
    <Button
      type="button"
      variant={iconOnly ? "ghost" : primary ? "default" : "outline"}
      size={iconOnly ? "icon-sm" : "default"}
      className={className}
      title={title}
      aria-label={iconOnly ? "Download" : undefined}
      aria-busy={pending}
      data-action={management ? "download" : "public-download"}
      data-resource={resource}
      onClick={() =>
        void run(async (signal) => {
          await download({
            shareId: share.id,
            resource,
            management,
            scope: management ? csrf : "public",
            password,
            signal,
            onPrepared,
          });
          if (!management) notify("");
        })
      }
    >
      <Download aria-hidden="true" />
      {iconOnly ? null : children || "Download"}
    </Button>
  );
}
