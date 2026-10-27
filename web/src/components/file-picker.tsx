import { useEffect, useRef, useState } from "react";
import {
  Check,
  CircleAlert,
  Clock3,
  FileQuestion,
  LoaderCircle,
  Plus,
  RotateCcw,
  Trash2,
  Undo2,
  Upload,
  X,
} from "lucide-react";
import { useApp } from "@/app/context";
import { bytes, fileCount } from "@/lib/format";
import {
  collectionErrors,
  fileError,
  type UploadItem,
} from "@/transfers/upload-manager";
import type { Limits } from "@/types/api";
import { Label } from "./ui/label";
import { Button } from "./ui/button";
import { Progress } from "./ui/progress";
import { Alert, AlertDescription } from "./ui/alert";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { IconButton } from "./icon-button";
import { FileThumbnail, ImageViewer } from "./file-preview";

function screenshotName(file: File, index: number) {
  const now = new Date();
  const stamp = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")} ${[now.getHours(), now.getMinutes(), now.getSeconds()].map((part) => String(part).padStart(2, "0")).join("-")}`;
  const extension =
    (
      {
        "image/jpeg": "jpg",
        "image/webp": "webp",
        "image/gif": "gif",
        "image/avif": "avif",
      } as Record<string, string>
    )[file.type] ?? "png";
  return `Screenshot ${stamp}${index ? ` (${index + 1})` : ""}.${extension}`;
}
export function FilePicker({
  collectionKey,
  limits,
  onAdded,
}: {
  collectionKey: string;
  limits?: Limits;
  onAdded?: () => void;
}) {
  const { uploads, notify } = useApp();
  const value = uploads.get(collectionKey);
  const [dragging, setDragging] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const disabled =
    !value ||
    value.creating ||
    value.publishing ||
    (!!value.share &&
      (value.share.state !== "draft" ||
        value.share.created + 86400 <= Date.now() / 1000));
  function add(files: File[]) {
    if (!files.length) return;
    const renamed = uploads.add(collectionKey, files);
    if (renamed)
      notify(
        `${renamed === 1 ? "A file was" : `${renamed} files were`} given numbered names to keep both copies.`,
      );
    onAdded?.();
  }
  useEffect(() => {
    if (disabled) return;
    const paste = (event: ClipboardEvent) => {
      const target = event.target;
      if (
        (target instanceof Element &&
          target.closest(
            "textarea, [contenteditable]:not([contenteditable='false']), [role='textbox']",
          )) ||
        (target instanceof HTMLInputElement &&
          ![
            "file",
            "button",
            "submit",
            "reset",
            "checkbox",
            "radio",
            "range",
            "color",
          ].includes(target.type))
      )
        return;
      if (document.querySelector('[data-slot="dialog-content"]')) return;
      const files = Array.from(event.clipboardData?.items ?? [])
        .filter(
          (item) => item.kind === "file" && item.type.startsWith("image/"),
        )
        .map((item) => item.getAsFile())
        .filter((file): file is File => !!file)
        .map(
          (file, index) =>
            new File([file], screenshotName(file, index), {
              type: file.type,
              lastModified: Date.now(),
            }),
        );
      if (!files.length) return;
      event.preventDefault();
      const renamed = uploads.add(collectionKey, files);
      notify(
        `Pasted ${fileCount(files.length)}${renamed ? " with numbered names" : ""}.`,
      );
      onAdded?.();
    };
    document.addEventListener("paste", paste);
    return () => document.removeEventListener("paste", paste);
  }, [collectionKey, disabled, notify, onAdded, uploads]);
  if (!value) return null;
  const items = value.items.filter((item) => !item.removedAt);
  const removed = value.items.filter((item) => item.removedAt);
  const errors = collectionErrors(value, limits);
  const size = items.reduce((sum, item) => sum + item.size, 0);
  return (
    <div className="file-collection">
      <Label
        id="drop-zone"
        className={`drop-zone${items.length ? " has-files" : ""}${dragging && !disabled ? " dragging" : ""}${disabled ? " disabled" : ""}`}
        onDragOver={(event) => {
          event.preventDefault();
          if (event.dataTransfer.types.includes("Files")) setDragging(true);
        }}
        onDragLeave={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null))
            setDragging(false);
        }}
        onDrop={(event) => {
          event.preventDefault();
          setDragging(false);
          if (disabled) return;
          const entries = Array.from(event.dataTransfer.items);
          if (entries.some((item) => item.webkitGetAsEntry?.()?.isDirectory))
            notify("Choose the files inside a folder to add them.", true);
          add(
            entries.length
              ? entries
                  .filter((item) => !item.webkitGetAsEntry?.()?.isDirectory)
                  .map((item) => item.getAsFile())
                  .filter((file): file is File => !!file)
              : Array.from(event.dataTransfer.files),
          );
        }}
      >
        {items.length ? (
          <Plus aria-hidden="true" />
        ) : (
          <Upload aria-hidden="true" />
        )}
        <strong>
          {items.length
            ? "Drop more files, or choose files"
            : "Drop files here, or choose files"}
        </strong>
        <span>
          Images, documents, and more. You can also paste screenshots.
        </span>
        {limits && (
          <span className="fine">
            Up to {limits.settings.file_count} files ·{" "}
            {bytes(limits.settings.file_limit)} per file
          </span>
        )}
        <input
          id="files"
          type="file"
          multiple
          aria-label="Choose files"
          disabled={disabled}
          onChange={(event) => {
            add(Array.from(event.target.files ?? []));
            event.target.value = "";
          }}
        />
      </Label>
      {!!items.length && (
        <div className="file-collection-tools">
          <p className="fine muted" aria-live="polite">
            {fileCount(items.length)} · {bytes(size)} total
          </p>
          <IconButton
            label="Clear all files"
            disabled={disabled || !!value.pending}
            onClick={() => setConfirming(true)}
          >
            <Trash2 aria-hidden="true" />
          </IconButton>
        </div>
      )}
      <ul
        id="selected-files"
        className="file-queue"
        aria-label="Files to share"
      >
        {items.map((item) => (
          <FileRow
            key={item.key}
            item={item}
            collectionKey={collectionKey}
            disabled={disabled}
            validation={fileError(item, limits)}
            onPreview={() => setPreview(item.key)}
          />
        ))}
      </ul>
      <div className="removal-feedback" aria-live="polite">
        {removed.map((item) => (
          <div className="undo-row" key={item.key}>
            <span title={item.name}>{item.name} removed</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={!!value.pending}
              onClick={() => void uploads.undo(collectionKey, item.key)}
            >
              <Undo2 aria-hidden="true" />
              Undo<span className="sr-only"> removal of {item.name}</span>
            </Button>
          </div>
        ))}
      </div>
      {!!errors.length && (
        <Alert variant="destructive" className="mt-4">
          <CircleAlert aria-hidden="true" />
          <AlertDescription>
            {errors.map((error) => (
              <p key={error}>{error}</p>
            ))}
          </AlertDescription>
        </Alert>
      )}
      {value.error && !errors.includes(value.error) && (
        <Alert variant="destructive" className="mt-4">
          <CircleAlert aria-hidden="true" />
          <AlertDescription>{value.error}</AlertDescription>
        </Alert>
      )}
      <ImageViewer
        items={value.items}
        selected={preview}
        onSelect={setPreview}
      />
      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Clear all files?</DialogTitle>
            <DialogDescription>
              Remove {fileCount(items.length)} from this private collection. You
              can briefly undo removals.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose render={<Button type="button" variant="outline" />}>
              Keep files
            </DialogClose>
            <Button
              type="button"
              variant="destructive"
              onClick={() => {
                void uploads.remove(
                  collectionKey,
                  items.map((item) => item.key),
                );
                setConfirming(false);
              }}
            >
              <Trash2 aria-hidden="true" />
              Clear all
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
function FileRow({
  item,
  collectionKey,
  disabled,
  validation,
  onPreview,
}: {
  item: UploadItem;
  collectionKey: string;
  disabled: boolean;
  validation?: string;
  onPreview: () => void;
}) {
  const { uploads } = useApp();
  const input = useRef<HTMLInputElement>(null);
  const state = item.state;
  const status =
    validation ??
    item.error ??
    {
      queued: "Queued",
      checking: "Checking file…",
      uploading: "Uploading…",
      uploaded: "Uploaded",
      "needs-file": "Needs file selection",
      failed: "Failed",
    }[state];
  const StateIcon =
    validation || state === "failed"
      ? CircleAlert
      : state === "uploaded"
        ? Check
        : state === "needs-file"
          ? FileQuestion
          : state === "checking" || state === "uploading"
            ? LoaderCircle
            : Clock3;
  return (
    <li
      className={`queued-file${validation || item.error ? " invalid" : ""}`}
      data-state={state}
    >
      <FileThumbnail item={item} onPreview={onPreview} />
      <div className="queued-file-content">
        <strong title={item.name}>{item.name}</strong>
        <div className="file-metadata">
          <span>{bytes(item.size)}</span>
          {item.local && item.local.name !== item.name && (
            <span title={item.local.name}>Renamed from {item.local.name}</span>
          )}
        </div>
        <div className={`file-state${state === "uploaded" ? " complete" : ""}`}>
          <StateIcon
            aria-hidden="true"
            className={
              state === "checking" || state === "uploading"
                ? "animate-spin"
                : undefined
            }
          />
          <span>
            {status}
            {(state === "uploading" || state === "needs-file") &&
            !!item.stored?.offset
              ? ` · ${bytes(item.stored.offset)} of ${bytes(item.size)}`
              : ""}
          </span>
        </div>
        {(state === "uploading" || state === "checking") && (
          <Progress
            aria-label={`Upload progress for ${item.name}`}
            value={
              item.size ? ((item.stored?.offset ?? 0) / item.size) * 100 : 0
            }
          />
        )}
      </div>
      <div className="file-actions">
        {(state === "needs-file" || state === "failed") && (
          <>
            <IconButton
              label={`Select original file for ${item.name}`}
              disabled={disabled}
              onClick={() => input.current?.click()}
            >
              <Upload aria-hidden="true" />
            </IconButton>
            <input
              ref={input}
              type="file"
              tabIndex={-1}
              className="sr-only"
              aria-label={`Original file for ${item.name}`}
              disabled={disabled}
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) uploads.attach(collectionKey, item.key, file);
                event.target.value = "";
              }}
            />
          </>
        )}
        {state === "failed" && item.local && !validation && (
          <IconButton
            label={`Retry ${item.name}`}
            disabled={disabled}
            onClick={() => uploads.retry(collectionKey, item.key)}
          >
            <RotateCcw aria-hidden="true" />
          </IconButton>
        )}
        <IconButton
          label={`Remove ${item.name}`}
          disabled={disabled}
          onClick={() => void uploads.remove(collectionKey, [item.key])}
        >
          <X aria-hidden="true" />
        </IconButton>
      </div>
    </li>
  );
}
