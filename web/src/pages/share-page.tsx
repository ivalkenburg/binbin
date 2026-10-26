import {
  RefreshError,
  Panel,
  Heading,
  Loading,
  Status,
  statusLabels,
} from "@/components/common";
import { LinkButton } from "@/components/link-button";
import { useRef, useState } from "react";
import { File } from "lucide-react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { useTask } from "@/hooks/use-task";
import { api } from "@/lib/api";
import {
  bytes,
  date,
  fileCount,
  localDate,
  optionalNumber,
} from "@/lib/format";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { CheckField, Field } from "@/components/forms";
import { FilePicker } from "@/components/file-picker";
import { UploadStatus } from "@/components/upload-status";
import { DownloadButton } from "@/components/download-controls";
import type { Share } from "@/types/api";
export function SharePage({ id }: { id: string }) {
  const { data: share, error, reload } = useRemote<Share>(`/api/shares/${id}`);
  const [files, setFiles] = useState<globalThis.File[]>([]);
  const { busy, upload, notify } = useApp();
  const { run, pending } = useTask();
  if (!share) return <Loading error={error} />;
  const downloadable = ["active", "exhausted"].includes(share.state);
  return (
    <>
      <a className="back" href="#shares">
        All shares
      </a>
      <Heading
        title={share.title}
        description={`${fileCount(share.files.length)} · ${bytes(share.original_size)} · Created ${date(share.created)}`}
      >
        <Status state={share.state} />
      </Heading>
      <RefreshError error={error} retry={reload} />
      <div className="share-detail-layout">
        <Panel className="work-area" title="Files">
          {share.files.map((file) => (
            <div className="detail-file" key={file.id}>
              <File aria-hidden="true" />
              <div>
                <strong title={file.name}>{file.name}</strong>
                <small>
                  {bytes(file.size)}
                  {share.state === "draft"
                    ? ` · ${bytes(file.offset)} uploaded`
                    : ""}
                </small>
              </div>
              {downloadable && (
                <DownloadButton share={share} resource={file.id} management />
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
          {share.state === "draft" && (
            <div className="draft-controls">
              {share.files.every((f) => f.offset === f.size) ? (
                <>
                  <Button
                    variant="default"
                    data-action="publish"
                    disabled={pending}
                    onClick={() =>
                      void run(async (signal) => {
                        await api(`/api/shares/${id}/publish`, {
                          method: "POST",
                          signal,
                        });
                        reload();
                        notify(
                          "Your share is ready. Copy the link to send it.",
                        );
                      })
                    }
                  >
                    {pending
                      ? "Preparing ZIP and publishing…"
                      : "Publish share"}
                  </Button>
                  <p className="fine muted">
                    The expiry clock starts when you publish.
                  </p>
                </>
              ) : (
                <>
                  <h3>Resume this upload</h3>
                  <p className="muted">
                    Select the original files. Drafts are kept for 24 hours from
                    creation.
                  </p>
                  <form
                    id="resume-form"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void upload(
                        (signal) => api<Share>(`/api/shares/${id}`, { signal }),
                        [...files],
                      );
                    }}
                  >
                    <FilePicker
                      files={files}
                      onChange={setFiles}
                      disabled={busy}
                    />
                    <Button type="submit" variant="default" disabled={busy}>
                      Resume upload
                    </Button>
                  </form>
                </>
              )}
              <UploadStatus />
            </div>
          )}
        </Panel>
        <ShareSettings share={share} reload={reload} />
      </div>
    </>
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
  const { notify } = useApp();
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
            Cancel changes
          </Button>
        )}
      </div>
      <div id="share-overview" hidden={editing}>
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
          <>
            <Field
              label="Share link"
              id="share-url"
              ref={link}
              readOnly
              value={s.url}
            />
            <Button
              className="full"
              variant="default"
              data-action="copy"
              onClick={() =>
                void run(async () => {
                  link.current?.select();
                  if (navigator.clipboard)
                    await navigator.clipboard.writeText(s.url);
                  else document.execCommand("copy");
                  notify("Copied to clipboard.");
                })
              }
            >
              Copy link
            </Button>
            <LinkButton
              variant="outline"
              className="full"
              href={s.url}
              target="_blank"
              rel="noopener"
            >
              Open public page
            </LinkButton>
            <Button
              variant="outline"
              className="full"
              data-action="edit"
              onClick={() => setEditing(true)}
            >
              Edit settings
            </Button>
          </>
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
              Save changes
            </Button>
          </form>
        </div>
      )}
      {!editing &&
        (["active", "draft", "exhausted"].includes(s.state) ? (
          <div className="danger-zone">
            <Button
              className="full"
              variant="destructive"
              data-action="confirm-delete"
              onClick={() => {
                notify("");
                setConfirming(true);
              }}
            >
              Revoke and delete
            </Button>
            {confirming && (
              <Alert variant="destructive" id="delete-confirm">
                <AlertDescription>
                  This permanently removes access and files. There is no undo.
                </AlertDescription>
                <Button
                  variant="destructive"
                  data-action="delete"
                  disabled={pending}
                  onClick={() =>
                    void run(async (signal) => {
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
                  Confirm deletion
                </Button>
                <Button
                  variant="outline"
                  data-action="keep"
                  onClick={() => setConfirming(false)}
                >
                  Keep share
                </Button>
              </Alert>
            )}
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
