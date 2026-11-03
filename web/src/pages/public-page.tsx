import { FileTypeIcon } from "@/components/file-preview";
import { LinkButton } from "@/components/link-button";
import { Panel, Loading } from "@/components/common";
import { useCallback, useEffect, useRef, useState } from "react";
import { File, Download, RotateCcw, Unlock } from "lucide-react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { useTask } from "@/hooks/use-task";
import { ApiError } from "@/lib/api";
import { bytes, date, fileCount } from "@/lib/format";
import {
  recoverPublicDownloads,
  retryRecoveredDownload,
  observeDownload,
  forgetPublicDownloads,
  type DownloadState,
  type RecoveredDownload,
} from "@/transfers/download";
import { Field } from "@/components/forms";
import { FileList } from "@/components/file-list";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { DownloadButton } from "@/components/download-controls";
import type { PublicShare } from "@/types/api";
export function PublicPage() {
  const shareId = location.pathname.split("/")[2];
  const [password, setPassword] = useState<string | null>(null);
  const lastPasswordError = useRef<unknown>(undefined);
  const {
    data: share,
    error,
    reload,
  } = useRemote<PublicShare>(`/api/public/${encodeURIComponent(shareId)}`, {
    password,
  });
  const { notify } = useApp();
  const { run } = useTask();
  const [recovered, setRecovered] = useState<RecoveredDownload[]>([]);
  const [checkingRetries, setCheckingRetries] = useState(false);
  const [retryError, setRetryError] = useState("");
  useEffect(() => {
    setRecovered([]);
    setRetryError("");
    setCheckingRetries(false);
    if (!(error instanceof ApiError) || ![404, 410].includes(error.status))
      return;
    const controller = new AbortController();
    setCheckingRetries(true);
    recoverPublicDownloads(shareId, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setRecovered(value);
      })
      .catch((error) => {
        if (!controller.signal.aborted)
          setRetryError(
            error instanceof Error
              ? error.message
              : "Couldn’t check previous downloads.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setCheckingRetries(false);
      });
    return () => controller.abort();
  }, [error, shareId]);
  useEffect(() => {
    if (share) document.title = `${share.title} · Binbin`;
  }, [share]);
  useEffect(() => {
    if (
      error instanceof ApiError &&
      error.status === 401 &&
      lastPasswordError.current !== error
    ) {
      lastPasswordError.current = error;
      if (password)
        notify("Incorrect password. Ask the sender or try again.", true);
    }
  }, [error, password, notify]);
  if (error) {
    if (error instanceof ApiError && error.status === 401)
      return (
        <Panel className="recipient-share">
          <h1>Password required</h1>
          <p className="muted">Enter the password provided by the sender.</p>
          <form
            id="share-password"
            onSubmit={(event) => {
              event.preventDefault();
              const value = String(
                new FormData(event.currentTarget).get("password"),
              );
              notify("");
              if (value === password) reload();
              else setPassword(value);
            }}
          >
            <Field
              label="Share password"
              name="password"
              type="password"
              autoComplete="off"
              maxLength={1024}
              required
              autoFocus
            />
            <Button variant="default" className="full" type="submit">
              <Unlock aria-hidden="true" />
              Open share
            </Button>
          </form>
        </Panel>
      );
    if (!(error instanceof ApiError) || ![404, 410].includes(error.status))
      return (
        <Panel className="recipient-share">
          <h1>Couldn’t load this share</h1>
          <p className="muted">
            Check your connection and try again. The server may be temporarily
            unavailable.
          </p>
          <Button variant="default" onClick={reload}>
            <RotateCcw aria-hidden="true" />
            Try again
          </Button>
        </Panel>
      );
    if (recovered.length)
      return (
        <Panel className="recipient-share">
          <h1>Resume your download</h1>
          <p className="muted">
            This share no longer accepts new downloads. Your started transfers
            can still be retried.
          </p>
          {recovered.map((item) => (
            <div className="detail-file" key={item.resource}>
              <FileTypeIcon name={item.label} />
              <div>
                <a
                  className="file-open-link"
                  href={`/d/${encodeURIComponent(item.token)}?inline=true`}
                  target="_blank"
                  rel="noopener"
                  aria-label={`Open ${item.label} in browser (new tab)`}
                  title={`${item.label} · Open in browser`}
                >
                  <strong>{item.label}</strong>
                </a>
                <small>Previously started transfer</small>
              </div>
              <LinkButton
                href={`/d/${encodeURIComponent(item.token)}`}
                download
                variant="default"
                onClick={(event) => {
                  event.preventDefault();
                  void run((signal) =>
                    retryRecoveredDownload(item.token, signal),
                  );
                }}
              >
                <Download aria-hidden="true" />
                Retry download
              </LinkButton>
            </div>
          ))}
          <p className="fine muted">
            Retries use the original transfer and don’t use another download.
          </p>
        </Panel>
      );
    return (
      <Panel className="recipient-share">
        <h1>Share unavailable</h1>
        <p className="muted">
          It may have expired or been removed. Ask the sender for a new link.
        </p>
        {checkingRetries && (
          <p className="fine muted" role="status">
            Checking previous downloads…
          </p>
        )}
        {retryError && (
          <>
            <p className="form-error" role="alert">
              {retryError}
            </p>
            <Button variant="outline" onClick={reload}>
              <RotateCcw aria-hidden="true" />
              Try again
            </Button>
          </>
        )}
      </Panel>
    );
  }
  if (!share) return <Loading />;
  if (share.delete_after_download)
    return <DeleteAfterDownload share={share} password={password} />;
  return (
    <Panel className="recipient-share">
      <div className="login-mark">
        <File aria-hidden="true" />
      </div>
      <h1>{share.title}</h1>
      <p className="muted">
        {fileCount(share.files.length)} ·{" "}
        {bytes(share.files.reduce((n, f) => n + f.size, 0))}
      </p>
      <FileList share={share} password={password} />
      <div className="recipient-limits">
        <span>
          {share.expires
            ? `Available until ${date(share.expires)}`
            : "No deadline"}
        </span>
        <span>
          {share.remaining === null
            ? "Unlimited downloads"
            : `${share.remaining} downloads remaining`}
        </span>
      </div>
    </Panel>
  );
}

function DeleteAfterDownload({
  share,
  password,
}: {
  share: PublicShare;
  password: string | null;
}) {
  const [state, setState] = useState<DownloadState | "blocked">(
    share.downloading ? "blocked" : "available",
  );
  const [checking, setChecking] = useState(true);
  const observer = useRef<AbortController | null>(null);
  const watch = useCallback(
    async (href: string) => {
      observer.current?.abort();
      const controller = new AbortController();
      observer.current = controller;
      await observeDownload(`${href}/status`, controller.signal, (next) => {
        if (controller.signal.aborted) return;
        if (next === "deleted" || next === "unavailable") {
          forgetPublicDownloads(share.id);
          document.title = `${next === "deleted" ? "Deleted after download" : "Share unavailable"} · Binbin`;
        }
        setState(next);
      });
    },
    [share.id],
  );
  useEffect(() => {
    const controller = new AbortController();
    void recoverPublicDownloads(share.id, controller.signal)
      .then(async (saved) => {
        if (!controller.signal.aborted && saved.length)
          await watch(`/d/${encodeURIComponent(saved[0].token)}`);
      })
      .catch(() => {
        if (!controller.signal.aborted) setState("connection-error");
      })
      .finally(() => {
        if (!controller.signal.aborted) setChecking(false);
      });
    return () => {
      controller.abort();
      observer.current?.abort();
    };
  }, [share.id, watch]);
  if (state === "deleted" || state === "unavailable")
    return (
      <Panel className="recipient-share">
        <h1>
          {state === "deleted" ? "Deleted after download" : "Share unavailable"}
        </h1>
        <p className="muted">
          {state === "deleted"
            ? "The files and share details have been removed. Check your browser’s downloads for your copy."
            : "It may have expired or been removed. Ask the sender for a new link."}
        </p>
      </Panel>
    );
  const single = share.files.length === 1;
  const resource = single ? share.files[0].id : "zip";
  return (
    <Panel className="recipient-share">
      <Badge variant="secondary">Delete after download</Badge>
      <h1>{share.title}</h1>
      <p className="muted">
        {fileCount(share.files.length)} ·{" "}
        {bytes(share.files.reduce((n, f) => n + f.size, 0))}
      </p>
      <div className="delete-download-notice">
        <strong>Download once. Then this share is deleted.</strong>
        <p className="fine muted">
          The files and share details are removed after the server finishes
          sending {single ? "the file" : "the ZIP"}.
        </p>
      </div>
      {checking ? (
        <p role="status">Checking previous downloads…</p>
      ) : state === "blocked" ? (
        <>
          <h2>Download already in progress</h2>
          <p className="muted">
            This share is reserved for another download. Check again later.
          </p>
          <Button variant="outline" onClick={() => location.reload()}>
            Check again
          </Button>
        </>
      ) : state === "downloading" || state === "deleting" ? (
        <div role="status">
          <h2>
            {state === "deleting" ? "Deleting share" : "Download in progress"}
          </h2>
          <p className="muted">
            {state === "deleting"
              ? "Access is closed. The server is removing the files and share details."
              : "Check your browser’s downloads. Other recipients cannot start while yours is active."}
          </p>
        </div>
      ) : state === "connection-error" ? (
        <>
          <p role="alert">
            Couldn’t check download status. Your browser’s download may still be
            running.
          </p>
          <Button variant="outline" onClick={() => location.reload()}>
            Check again
          </Button>
        </>
      ) : (
        <>
          {state === "interrupted" && (
            <div role="status">
              <h2>Continue your download</h2>
              <p className="fine muted">
                Resume in your browser’s downloads, or retry here. This
                browser’s transfer is reserved for 15 minutes after
                interruption; then the link becomes available again.
              </p>
            </div>
          )}
          <FileList share={share} downloadable={false} />
          <DownloadButton
            share={share}
            resource={resource}
            password={password}
            primary
            className="full"
            onPrepared={async (href) => {
              await watch(href);
              setState("downloading");
            }}
          >
            {state === "interrupted"
              ? "Retry download"
              : single
                ? "Download file once"
                : "Download ZIP once"}
          </DownloadButton>
        </>
      )}
      <div className="recipient-limits">
        <span>
          {share.expires
            ? `Available until ${date(share.expires)}`
            : "No deadline"}
        </span>
        <span>One complete download</span>
      </div>
    </Panel>
  );
}
