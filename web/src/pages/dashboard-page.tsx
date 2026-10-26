import {
  RefreshError,
  Heading,
  Loading,
  Status,
  statusLabels,
} from "@/components/common";
import { LinkButton } from "@/components/link-button";
import { Progress } from "@/components/ui/progress";
import { useState } from "react";
import { File, Upload, Plus } from "lucide-react";
import { useRemote } from "@/hooks/use-remote";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { bytes, date, fileCount } from "@/lib/format";
import type { Limits, Share } from "@/types/api";
export function DashboardPage() {
  const shares = useRemote<Share[]>("/api/shares");
  const limits = useRemote<Limits>("/api/settings");
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("");
  if (!shares.data || !limits.data)
    return <Loading error={shares.error || limits.error} />;
  const filtered = shares.data.filter(
    (s) =>
      s.title.toLowerCase().includes(search.toLowerCase()) &&
      (!status || s.state === status),
  );
  return (
    <>
      <Heading
        title="Your shares"
        description="Manage your files and the links you’ve shared."
      >
        <LinkButton
          variant="default"

          href="#new"
        >
          <Plus aria-hidden="true" />
          New share
        </LinkButton>
      </Heading>
      <RefreshError
        error={shares.error || limits.error}
        retry={() => {
          shares.reload();
          limits.reload();
        }}
      />
      <div className="storage-line">
        <span>
          {bytes(limits.data.used)} of{" "}
          {bytes(limits.data.settings.storage_limit)} reserved
        </span>
        <Progress
          aria-label="Storage usage"
          value={(limits.data.used / limits.data.settings.storage_limit) * 100}
        />
        <span>Drafts and ZIPs included</span>
      </div>
      <div className="list-tools">
        <Label className="search">
          <span className="sr-only">Search shares</span>
          <Input
            id="search"
            type="search"
            placeholder="Search by title…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </Label>
        <Label>
          <span className="sr-only">Filter by status</span>
          <NativeSelect
            id="status-filter"
            value={status}
            onChange={(e) => setStatus(e.target.value)}
          >
            <option value="">All shares</option>
            {["active", "draft", "expired", "exhausted", "revoked"].map((s) => (
              <option key={s} value={s}>
                {statusLabels[s]}
              </option>
            ))}
          </NativeSelect>
        </Label>
      </div>
      <section className="share-list" aria-label="Shares">
        <div className="list-head">
          <span>Share</span>
          <span>Size</span>
          <span>Downloads</span>
          <span>Deadline</span>
          <span>Status</span>
        </div>
        {filtered.map((s) => (
          <a className="share-row" href={`#share/${s.id}`} key={s.id}>
            <div className="file-title">
              <File aria-hidden="true" />
              <span>
                <strong>{s.title}</strong>
                <small>
                  {fileCount(s.files.length)}
                  {s.password_protected ? " · Password protected" : ""}
                </small>
              </span>
            </div>
            <span className="size">{bytes(s.original_size)}</span>
            <span className="downloads">
              {s.used} / {s.limit ?? "unlimited"}
            </span>
            <span className="deadline">
              {s.state === "draft" ? "Unpublished" : date(s.expires)}
            </span>
            <Status state={s.state} />
          </a>
        ))}
        {!shares.data.length && (
          <div className="empty">
            <Upload aria-hidden="true" />
            <h2>No shares yet.</h2>
            <p>Upload files and create your first share link.</p>
            <LinkButton
              variant="default"

              href="#new"
            >
              {" "}
              Create a share
            </LinkButton>
          </div>
        )}
        <p
          id="no-results"
          className="empty"
          hidden={filtered.length > 0 || !shares.data.length}
        >
          No shares match your search.
        </p>
      </section>
      <p className="fine muted">
        Public downloads count toward the allowance. Your management downloads
        do not.
      </p>
    </>
  );
}
