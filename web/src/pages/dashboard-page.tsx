import { FileTypeIcon } from "@/components/file-preview";
import {
  RefreshError,
  Heading,
  Loading,
  Status,
  statusLabels,
} from "@/components/common";
import { LinkButton } from "@/components/link-button";
import { IconButton } from "@/components/icon-button";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { Copy, Files, Upload, Plus, Search } from "lucide-react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { copyText } from "@/lib/clipboard";
import { errorMessage } from "@/lib/api";
import { DRAFT_LIFETIME, bytes, date, fileCount, timeLeft } from "@/lib/format";
import type { Share } from "@/types/api";
const sorters: Record<string, [string, (a: Share, b: Share) => number]> = {
  newest: ["Newest first", (a, b) => b.created - a.created],
  oldest: ["Oldest first", (a, b) => a.created - b.created],
  title: ["Title A–Z", (a, b) => a.title.localeCompare(b.title)],
  size: ["Largest first", (a, b) => b.original_size - a.original_size],
  deadline: [
    "Deadline soonest",
    (a, b) =>
      (a.expires ?? Infinity) - (b.expires ?? Infinity) ||
      b.created - a.created,
  ],
};
const deadline = (s: Share) =>
  s.state === "draft"
    ? `Draft · ${timeLeft(s.created + DRAFT_LIFETIME)}`
    : date(s.expires);
export function DashboardPage() {
  const shares = useRemote<Share[]>("/api/shares");
  const { notify } = useApp();
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("");
  const [sort, setSort] = useState("newest");
  if (!shares.data) return <Loading error={shares.error} />;
  const counts: Record<string, number> = {};
  for (const s of shares.data) counts[s.state] = (counts[s.state] ?? 0) + 1;
  const states = ["active", "draft", "expired", "exhausted", "revoked"].filter(
    (state) => counts[state],
  );
  const chips: [string, string, number][] = [
    ["", "All shares", shares.data.length],
    ...states.map((s): [string, string, number] => [
      s,
      statusLabels[s],
      counts[s],
    ]),
  ];
  const filtered = shares.data
    .filter(
      (s) =>
        s.title.toLowerCase().includes(search.toLowerCase()) &&
        (!status || s.state === status),
    )
    .sort(sorters[sort][1]);
  return (
    <>
      <Heading
        title="Your shares"
        description="Manage your files and the links you’ve shared."
      >
        <LinkButton variant="default" href="#new">
          <Plus aria-hidden="true" />
          New share
        </LinkButton>
      </Heading>
      <RefreshError error={shares.error} retry={shares.reload} />
      {!!shares.data.length && (
        <>
          <div className="list-tools">
            <Label className="search">
              <span className="sr-only">Search shares</span>
              <Search className="search-icon" aria-hidden="true" />
              <Input
                id="search"
                type="search"
                placeholder="Search by title…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </Label>
            <Label>
              <span className="sr-only">Sort shares</span>
              <NativeSelect
                id="sort"
                value={sort}
                onChange={(e) => setSort(e.target.value)}
              >
                {Object.entries(sorters).map(([key, [label]]) => (
                  <option key={key} value={key}>
                    {label}
                  </option>
                ))}
              </NativeSelect>
            </Label>
          </div>
          {states.length > 1 && (
            <div className="filter-chips" role="group" aria-label="Status">
              {chips.map(([key, label, count]) => (
                <Button
                  key={key}
                  type="button"
                  size="sm"
                  variant={status === key ? "secondary" : "ghost"}
                  aria-pressed={status === key}
                  data-status={key}
                  onClick={() => setStatus(key)}
                >
                  {label}
                  <span className="chip-count">{count}</span>
                </Button>
              ))}
            </div>
          )}
        </>
      )}
      <section className="share-list" aria-label="Shares">
        {!!shares.data.length && (
          <div className="list-head">
            <span>Share</span>
            <span>Size</span>
            <span>Downloads</span>
            <span>Deadline</span>
            <span>Status</span>
            <span />
          </div>
        )}
        {filtered.map((s) => (
          <div className="share-row" key={s.id}>
            <div className="file-title">
              {s.files.length === 1 ? (
                <FileTypeIcon name={s.files[0].name} />
              ) : (
                <Files aria-hidden="true" />
              )}
              <span>
                <a className="share-link" href={`#share/${s.id}`}>
                  <strong>{s.title}</strong>
                </a>
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
            <span className="deadline">{deadline(s)}</span>
            <Status state={s.state} />
            <span className="row-actions">
              {s.state === "active" && (
                <IconButton
                  label="Copy link"
                  size="icon-sm"
                  data-action="copy-row"
                  onClick={() =>
                    void copyText(s.url).then(
                      () => notify(`Link copied for “${s.title}”.`),
                      (error) => notify(errorMessage(error), true),
                    )
                  }
                >
                  <Copy aria-hidden="true" />
                </IconButton>
              )}
            </span>
          </div>
        ))}
        {!shares.data.length && (
          <div className="empty">
            <Upload aria-hidden="true" />
            <h2>No shares yet.</h2>
            <p>Upload files and create your first share link.</p>
            <LinkButton variant="default" href="#new">
              <Plus aria-hidden="true" />
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
    </>
  );
}
