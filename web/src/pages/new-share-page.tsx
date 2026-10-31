import { Panel, Heading, Loading } from "@/components/common";
import { useCallback, useEffect, useState } from "react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { Upload } from "lucide-react";
import { collectionErrors } from "@/transfers/upload-manager";
import { fileCount, optionalNumber } from "@/lib/format";
import { Field } from "@/components/forms";
import { FilePicker } from "@/components/file-picker";
import { Button } from "@/components/ui/button";
import type { Limits, NewShare } from "@/types/api";
export function NewSharePage() {
  const limits = useRemote<Limits>("/api/settings");
  const { uploads, navigate } = useApp();
  const collection = uploads.get("new")!;
  const [title, setTitle] = useState(collection.details?.title ?? "");
  const busy = collection.creating;
  const files = collection.items.filter((item) => !item.removedAt);
  useEffect(() => {
    if (limits.data) uploads.setLimits(limits.data);
  }, [limits.data, uploads]);
  const added = useCallback(() => {
    const files = uploads.get("new")!.items.filter((item) => !item.removedAt);
    const suggested =
      files.length === 1 ? files[0].name : fileCount(files.length);
    if (!uploads.get("new")!.details?.title)
      uploads.saveDetails({ ...uploads.get("new")!.details, title: suggested });
    setTitle(
      (current) =>
        current ||
        (files.length === 1 ? files[0].name : fileCount(files.length)),
    );
  }, [uploads]);
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
        onChange={(event) => {
          if (
            event.target instanceof HTMLInputElement &&
            event.target.type === "file"
          )
            return;
          uploads.saveDetails(
            Object.fromEntries(
              [...new FormData(event.currentTarget)].map(([key, value]) => [
                key,
                String(value),
              ]),
            ),
          );
        }}
        onSubmit={(event) => {
          event.preventDefault();
          const v = new FormData(event.currentTarget);
          const body: Omit<NewShare, "files"> = {
            title: String(v.get("title")),
            days: optionalNumber(v.get("days")),
            limit: optionalNumber(v.get("limit")),
            password: String(v.get("password") || "") || null,
          };
          void uploads.create(body, (id) => {
            if (location.hash === "#new") navigate(`share/${id}`);
          });
        }}
      >
        <div className="upload-layout">
          <Panel className="work-area" title="Files to share">
            <FilePicker
              collectionKey="new"
              limits={uploads.limits ?? limits.data}
              onAdded={added}
            />
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
                defaultValue={
                  collection.details?.days ?? settings.default_days ?? ""
                }
                max={settings.max_days ?? undefined}
                placeholder="No deadline"
                disabled={busy}
              />
              <Field
                label="Download allowance"
                name="limit"
                defaultValue={collection.details?.limit ?? ""}
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
              defaultValue={collection.details?.password ?? ""}
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
              disabled={
                busy ||
                !files.length ||
                !!collectionErrors(collection, uploads.limits ?? limits.data)
                  .length
              }
            >
              <Upload aria-hidden="true" />
              {busy ? "Creating draft…" : "Upload files"}
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
