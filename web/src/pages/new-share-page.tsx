import { Panel, Heading, Loading } from "@/components/common";
import { useState } from "react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { api } from "@/lib/api";
import { fileCount, optionalNumber } from "@/lib/format";
import { Field } from "@/components/forms";
import { FilePicker } from "@/components/file-picker";
import { UploadStatus } from "@/components/upload-status";
import { Button } from "@/components/ui/button";
import type { Limits, NewShare, Share } from "@/types/api";
export function NewSharePage() {
  const limits = useRemote<Limits>("/api/settings");
  const [files, setFiles] = useState<File[]>([]);
  const [title, setTitle] = useState("");
  const { busy, upload, notify } = useApp();
  if (!limits.data) return <Loading error={limits.error} />;
  const settings = limits.data.settings;
  return (
    <>
      <Heading
        title="New share"
        description="Upload files, set their limits, then publish a link."
      />
      <form
        id="new-share"
        onSubmit={(event) => {
          event.preventDefault();
          if (!files.length) {
            notify("Choose at least one file.", true);
            return;
          }
          const v = new FormData(event.currentTarget);
          const body: NewShare = {
            title: String(v.get("title")),
            files: files.map((f) => ({ name: f.name, size: f.size })),
            days: optionalNumber(v.get("days")),
            limit: optionalNumber(v.get("limit")),
            password: String(v.get("password") || "") || null,
          };
          void upload(
            (signal) =>
              api<Share>("/api/shares", { method: "POST", body, signal }),
            [...files],
          );
        }}
      >
        <div className="upload-layout">
          <Panel className="work-area" title="Files to share">
            <FilePicker
              files={files}
              disabled={busy}
              limits={settings}
              onChange={(next) => {
                setFiles(next);
                if (!title)
                  setTitle(
                    next.length === 1 ? next[0].name : fileCount(next.length),
                  );
              }}
            />
            <UploadStatus />
          </Panel>
          <Panel className="share-options" title="Share details">
            <Field
              label="Title"
              name="title"
              placeholder="e.g. Project handover"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              disabled={busy}
              required
              maxLength={200}
            />
            <div className="field-pair">
              <Field
                label="Expires after (days)"
                name="days"
                type="number"
                min={1}
                defaultValue={settings.default_days ?? ""}
                max={settings.max_days ?? undefined}
                placeholder="No deadline"
                disabled={busy}
              />
              <Field
                label="Download allowance"
                name="limit"
                type="number"
                min={1}
                max={settings.max_downloads ?? undefined}
                placeholder="Unlimited"
                disabled={busy}
              />
            </div>
            <p className="fine muted">
              Leave a limit blank for unlimited. The first limit reached ends
              the share.
            </p>
            <Field
              label={
                <>
                  Password <span className="optional">optional</span>
                </>
              }
              name="password"
              type="password"
              minLength={12}
              maxLength={1024}
              autoComplete="new-password"
              placeholder="At least 12 characters"
              disabled={busy}
            />
            <p className="fine muted">
              Each file download uses one allowance. The complete ZIP also uses
              one.
            </p>
            <Button
              className="full"
              variant="default"
              type="submit"
              disabled={busy}
            >
              Upload files
            </Button>
            <p className="fine muted">
              Files stay private until you publish. ZIPs require extra storage.
            </p>
          </Panel>
        </div>
      </form>
    </>
  );
}
