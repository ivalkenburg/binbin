import {
  RefreshError,
  Panel,
  Heading,
  Loading,
  Status,
  statusLabels,
} from "@/components/common";
import { LinkButton } from "@/components/link-button";
import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  Copy,
  ExternalLink,
  Pencil,
  Play,
  Save,
  Share2,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { useTask } from "@/hooks/use-task";
import { api } from "@/lib/api";
import {
  DRAFT_LIFETIME,
  bytes,
  date,
  fileCount,
  localDate,
  optionalNumber,
  timeLeft,
} from "@/lib/format";
import { copyText } from "@/lib/clipboard";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { CheckField, Field } from "@/components/forms";
import { FilePicker } from "@/components/file-picker";
import { DownloadButton, FileOpenButton } from "@/components/download-controls";
import { FileTypeIcon } from "@/components/file-preview";
import { collectionErrors } from "@/transfers/upload-manager";
import type { Limits, Share } from "@/types/api";
export function SharePage({ id }: { id: string }) {
  const { uploads } = useApp();
  const { data: remote, error, reload } = useRemote<Share>(`/api/shares/${id}`);
  useEffect(() => {
    if (remote) uploads.hydrate(remote);
  }, [remote, uploads]);
  const cached = uploads.get(id)?.share;
  const share =
    cached &&
    (!remote || remote.state === "draft" || cached.version > remote.version)
      ? cached
      : remote;
  if (!share) return <Loading error={error} />;
  const downloadable = ["active", "exhausted"].includes(share.state);
  return (
    <>
      <a className="back" href="#shares">
        <ArrowLeft aria-hidden="true" />
        All shares
      </a>
      <Heading
        title={share.title}
        description={`${fileCount(share.files.length)} · ${bytes(share.original_size)} · Created ${date(share.created)}`}
        titleSuffix={
          <>
            <Status state={share.state} className="heading-badge" />
            {share.state === "draft" && (
              <Badge
                variant="outline"
                className="heading-badge"
                title={`Draft expires ${date(share.created + DRAFT_LIFETIME)}`}
              >
                {timeLeft(share.created + DRAFT_LIFETIME)}
              </Badge>
            )}
          </>
        }
      />
      <RefreshError error={error} retry={reload} />
      <div className="share-detail-layout">
        <Panel className="work-area" title="Files">
          {share.state === "draft" ? (
            <DraftFiles share={share} />
          ) : (
            <>
              {share.files.map((file) => (
                <div className="detail-file" key={file.id}>
                  <FileTypeIcon name={file.name} />
                  <div>
                    {downloadable ? (
                      <FileOpenButton
                        share={share}
                        resource={file.id}
                        management
                      >
                        {file.name}
                      </FileOpenButton>
                    ) : (
                      <strong title={file.name}>{file.name}</strong>
                    )}
                    <small>{bytes(file.size)}</small>
                  </div>
                  {downloadable && (
                    <DownloadButton
                      share={share}
                      resource={file.id}
                      management
                    />
                  )}
                </div>
              ))}
              {!share.files.length && (
                <p className="muted">File contents have been removed.</p>
              )}
              {share.zip_hash && downloadable && (
                <DownloadButton
                  share={share}
                  resource="zip"
                  management
                  className="full"
                >
                  Download all as ZIP · {bytes(share.zip_size)}
                </DownloadButton>
              )}
            </>
          )}
        </Panel>
        <ShareSettings share={share} reload={reload} />
      </div>
      {share.state === "draft" && (
        <PublishShare share={share} reload={reload} />
      )}
    </>
  );
}
function DraftFiles({ share }: { share: Share }) {
  const { uploads } = useApp();
  const limits = useRemote<Limits>("/api/settings");
  useEffect(() => {
    if (limits.data) uploads.setLimits(limits.data);
  }, [limits.data, uploads]);
  const value = uploads.get(share.id);
  if (!value) return <Loading />;
  const items = value.items.filter((item) => !item.removedAt);
  const unfinished = items.filter((item) => item.state !== "uploaded");
  const selectable = unfinished.some(
    (item) => item.state === "needs-file" || !item.local,
  );
  const queued = unfinished.some(
    (item) => item.local && item.state === "queued",
  );
  const resuming =
    value.paused ||
    unfinished.some((item) => item.resuming || (item.stored?.offset ?? 0) > 0);
  const expired = share.created + DRAFT_LIFETIME <= Date.now() / 1000;
  return (
    <>
      {selectable && (
        <div className="resume-guidance">
          <h3>Resume this upload</h3>
          <p className="fine muted">
            Completed files are already saved. Select only the unfinished
            originals to resume from their saved progress.
          </p>
        </div>
      )}
      <FilePicker
        collectionKey={share.id}
        limits={uploads.limits ?? limits.data ?? undefined}
      />
      {!value.running && queued && (
        <div className="draft-controls">
          <Button
            type="button"
            variant="outline"
            disabled={
              expired ||
              !!value.pending ||
              !!collectionErrors(value, uploads.limits).length
            }
            onClick={() => void uploads.start(share.id)}
          >
            {resuming ? (
              <Play aria-hidden="true" />
            ) : (
              <Upload aria-hidden="true" />
            )}
            {resuming ? "Resume upload" : "Upload new files"}
          </Button>
        </div>
      )}
      {(expired || !!unfinished.length) && (
        <p className="fine muted">
          {expired
            ? "This draft has expired. Create a new share to upload these files."
            : "Upload or remove every unfinished file before publishing."}
        </p>
      )}
    </>
  );
}
function PublishShare({ share, reload }: { share: Share; reload: () => void }) {
  const { uploads, notify } = useApp();
  const value = uploads.get(share.id);
  const items = value?.items.filter((item) => !item.removedAt) ?? [];
  const uploaded = items.filter((item) => item.state === "uploaded").length;
  const ready =
    !!value &&
    !!items.length &&
    uploaded === items.length &&
    !value.running &&
    !value.pending &&
    !value.publishing;
  const expired = share.created + DRAFT_LIFETIME <= Date.now() / 1000;
  return (
    <div className="publish-bar">
      <span className="publish-summary" role="status">
        {uploaded} of {fileCount(items.length)} uploaded
      </span>
      <Button
        type="button"
        data-action="publish"
        disabled={!ready || expired}
        onClick={() =>
          void uploads
            .publish(share.id)
            .then(() => {
              reload();
              notify("Your share is ready. Copy the link to send it.");
            })
            .catch(() => {})
        }
      >
        <Share2 aria-hidden="true" />
        {value?.publishing ? "Preparing ZIP and publishing…" : "Publish share"}
      </Button>
    </div>
  );
}
function ShareSettings({
  share: s,
  reload,
}: {
  share: Share;
  reload: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const { run, pending } = useTask();
  const { notify, uploads } = useApp();
  const link = useRef<HTMLInputElement>(null);
  return (
    <Panel className="share-options">
      <div className="panel-heading">
        <h2 id="share-settings-title">
          {editing ? "Edit share" : "Share settings"}
        </h2>
        {editing && (
          <Button
            type="button"
            variant="ghost"
            data-action="cancel-edit"
            onClick={() => setEditing(false)}
          >
            <X aria-hidden="true" />
            Cancel changes
          </Button>
        )}
      </div>
      <div id="share-overview" hidden={editing}>
        {s.state === "active" && (
          <div className="share-link-block">
            <Field
              label="Share link"
              id="share-url"
              ref={link}
              readOnly
              value={s.url}
            />
            <div className="link-actions">
              <Button
                variant="default"
                data-action="copy"
                onClick={() =>
                  void run(async () => {
                    link.current?.select();
                    await copyText(s.url);
                    notify("Copied to clipboard.");
                  })
                }
              >
                <Copy aria-hidden="true" />
                Copy link
              </Button>
              <LinkButton
                variant="outline"
                href={s.url}
                target="_blank"
                rel="noopener"
              >
                <ExternalLink aria-hidden="true" />
                Open public page
              </LinkButton>
            </div>
          </div>
        )}
        <dl>
          {[
            [
              "Deadline",
              s.state === "draft" ? "Starts at publication" : date(s.expires),
            ],
            ["Downloads", `${s.used} / ${s.limit ?? "unlimited"}`],
            ["Storage reserved", bytes(s.reserved)],
            ["Password", s.password_protected ? "Required" : "None"],
          ].map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
        {s.state === "active" && (
          <Button
            variant="outline"
            className="full"
            data-action="edit"
            onClick={() => setEditing(true)}
          >
            <Pencil aria-hidden="true" />
            Edit settings
          </Button>
        )}
      </div>
      {editing && (
        <div id="edit-area">
          <form
            id="edit-share"
            onSubmit={(event) => {
              event.preventDefault();
              const values = new FormData(event.currentTarget);
              void run(async (signal) => {
                await api(`/api/shares/${s.id}`, {
                  method: "PATCH",
                  signal,
                  body: {
                    title: values.get("title"),
                    expires: values.get("expires")
                      ? Math.floor(
                          new Date(String(values.get("expires"))).getTime() /
                            1000,
                        )
                      : null,
                    limit: optionalNumber(values.get("limit")),
                    password: values.get("password") || null,
                    change_password: values.has("change_password"),
                  },
                });
                reload();
                setEditing(false);
                notify("Share settings saved.");
              });
            }}
          >
            <Field
              label="Title"
              name="title"
              defaultValue={s.title}
              maxLength={200}
              required
              autoFocus
            />
            <Field
              label="Deadline"
              name="expires"
              type="datetime-local"
              defaultValue={s.expires ? localDate(s.expires) : ""}
            />
            <Field
              label="Total download allowance"
              name="limit"
              type="number"
              min={1}
              defaultValue={s.limit ?? ""}
              placeholder="Unlimited"
            />
            <CheckField
              label="Replace or remove password"
              name="change_password"
            />
            <Field
              label="New password"
              name="password"
              type="password"
              minLength={12}
              maxLength={1024}
              autoComplete="new-password"
              placeholder="Blank removes password"
            />
            <p className="fine muted">
              Replacing the password invalidates existing transfer credentials.
            </p>
            <Button type="submit" variant="default" disabled={pending}>
              <Save aria-hidden="true" />
              Save changes
            </Button>
          </form>
        </div>
      )}
      {!editing &&
        (["active", "draft", "exhausted"].includes(s.state) ? (
          <div className="share-actions">
            <Button
              className="full"
              variant="destructive"
              data-action="confirm-delete"
              onClick={() => {
                notify("");
                setConfirming(true);
              }}
            >
              <Trash2 aria-hidden="true" />
              Revoke and delete
            </Button>
            <Dialog open={confirming} onOpenChange={setConfirming}>
              <DialogContent id="delete-confirm" showCloseButton={false}>
                <DialogHeader>
                  <DialogTitle>Delete “{s.title}”?</DialogTitle>
                  <DialogDescription>
                    This permanently removes access and files. There is no undo.
                  </DialogDescription>
                </DialogHeader>
                <DialogFooter>
                  <Button
                    variant="outline"
                    data-action="keep"
                    onClick={() => setConfirming(false)}
                  >
                    Keep share
                  </Button>
                  <Button
                    variant="destructive"
                    data-action="delete"
                    disabled={pending}
                    onClick={() =>
                      void run(async (signal) => {
                        uploads.pause(s.id);
                        await api(`/api/shares/${s.id}`, {
                          method: "DELETE",
                          signal,
                        });
                        setConfirming(false);
                        reload();
                        notify("Share removed.");
                      })
                    }
                  >
                    <Trash2 aria-hidden="true" />
                    Confirm deletion
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        ) : (
          <p className="muted">
            {s.reason || statusLabels[s.state]}. Minimal history is kept for 30
            days.
          </p>
        ))}
    </Panel>
  );
}
