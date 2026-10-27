import { useEffect, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileSpreadsheet,
  FileText,
  FileVideo,
  ImageOff,
  LoaderCircle,
  RotateCcw,
} from "lucide-react";
import type { UploadItem } from "@/transfers/upload-manager";
import { bytes } from "@/lib/format";
import { IconButton } from "./icon-button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";

export function FileTypeIcon({ name }: { name: string }) {
  const extension = name.split(".").at(-1)?.toLowerCase() ?? "";
  const Icon = /^(png|jpe?g|gif|webp|avif|svg|heic|bmp)$/.test(extension)
    ? FileImage
    : /^(zip|tar|gz|7z|rar|bz2|xz)$/.test(extension)
      ? FileArchive
      : /^(mp3|wav|flac|m4a|ogg|aac)$/.test(extension)
        ? FileAudio
        : /^(mp4|mov|webm|mkv|avi)$/.test(extension)
          ? FileVideo
          : /^(csv|xlsx?|ods)$/.test(extension)
            ? FileSpreadsheet
            : /^(js|jsx|ts|tsx|json|rs|py|html|css|sh|yaml|yml|xml)$/.test(
                  extension,
                )
              ? FileCode
              : /^(txt|md|pdf|docx?|odt|rtf)$/.test(extension)
                ? FileText
                : File;
  return <Icon aria-hidden="true" />;
}
export function isImage(item: Pick<UploadItem, "name" | "local">) {
  return (
    /\.(png|jpe?g|gif|webp|avif)$/i.test(item.name) ||
    /^(image\/(png|jpeg|gif|webp|avif))$/.test(item.local?.type ?? "")
  );
}
function useImageSource(item: UploadItem) {
  const local = item.local;
  const [source, setSource] = useState<{
    file: globalThis.File;
    url: string;
  } | null>(null);
  useEffect(() => {
    if (!local) return;
    const url = URL.createObjectURL(local);
    setSource({ file: local, url });
    return () => URL.revokeObjectURL(url);
  }, [local]);
  if (local) return source?.file === local ? source.url : undefined;
  if (item.stored?.offset === item.size)
    return `/api/files/${item.stored.id}/preview`;
}
function ImageContent({
  item,
  large = false,
}: {
  item: UploadItem;
  large?: boolean;
}) {
  const source = useImageSource(item);
  const [failed, setFailed] = useState<string>();
  const [attempt, setAttempt] = useState(0);
  if (!source || failed === source)
    return large ? (
      <div className="preview-unavailable" role="status">
        {!source && item.local ? (
          <LoaderCircle aria-hidden="true" className="animate-spin" />
        ) : (
          <ImageOff aria-hidden="true" />
        )}
        <p>
          {source
            ? "This image could not be previewed."
            : item.local
              ? "Loading preview…"
              : "Select the original file to preview this image."}
        </p>
        {source && (
          <IconButton
            label="Retry image preview"
            onClick={() => {
              setFailed(undefined);
              setAttempt((value) => value + 1);
            }}
          >
            <RotateCcw aria-hidden="true" />
          </IconButton>
        )}
      </div>
    ) : (
      <FileTypeIcon name={item.name} />
    );
  return (
    <img
      key={`${source}:${attempt}`}
      src={source}
      alt={large ? item.name : ""}
      loading={large ? "eager" : "lazy"}
      decoding="async"
      onError={() => setFailed(source)}
    />
  );
}
export function FileThumbnail({
  item,
  onPreview,
}: {
  item: UploadItem;
  onPreview: () => void;
}) {
  if (!isImage(item) || (!item.local && item.stored?.offset !== item.size))
    return (
      <span className="file-thumbnail">
        <FileTypeIcon name={item.name} />
      </span>
    );
  return (
    <IconButton
      className="file-thumbnail"
      variant="outline"
      label={`Preview ${item.name}`}
      onClick={onPreview}
    >
      <ImageContent item={item} />
    </IconButton>
  );
}
export function ImageViewer({
  items,
  selected,
  onSelect,
}: {
  items: UploadItem[];
  selected: string | null;
  onSelect: (key: string | null) => void;
}) {
  const images = items.filter(
    (item) =>
      !item.removedAt &&
      isImage(item) &&
      (item.local || item.stored?.offset === item.size),
  );
  const index = images.findIndex((item) => item.key === selected);
  const item = images[index];
  const move = (step: number) => {
    if (images[index + step]) onSelect(images[index + step].key);
  };
  return (
    <Dialog
      open={!!item}
      onOpenChange={(open) => {
        if (!open) onSelect(null);
      }}
    >
      <DialogContent
        className="image-viewer sm:max-w-4xl"
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft") {
            event.preventDefault();
            move(-1);
          }
          if (event.key === "ArrowRight") {
            event.preventDefault();
            move(1);
          }
        }}
      >
        <DialogHeader>
          <DialogTitle className="pr-8 break-words">{item?.name}</DialogTitle>
          <DialogDescription>
            {item
              ? `${bytes(item.size)} · Image ${index + 1} of ${images.length}`
              : "Image preview"}
          </DialogDescription>
        </DialogHeader>
        <div className="image-stage">
          {item && <ImageContent key={item.key} item={item} large />}
        </div>
        <div className="image-navigation">
          <IconButton
            label="Previous image"
            disabled={index <= 0}
            onClick={() => move(-1)}
          >
            <ChevronLeft aria-hidden="true" />
          </IconButton>
          <span className="fine muted">
            {index + 1} / {images.length}
          </span>
          <IconButton
            label="Next image"
            disabled={index >= images.length - 1}
            onClick={() => move(1)}
          >
            <ChevronRight aria-hidden="true" />
          </IconButton>
        </div>
      </DialogContent>
    </Dialog>
  );
}
