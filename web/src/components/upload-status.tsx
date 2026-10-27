import { LoaderCircle, Pause } from "lucide-react";
import { Progress } from "./ui/progress";
import { IconButton } from "./icon-button";
import { useApp } from "@/app/context";
import { bytes } from "@/lib/format";

export function UploadStatus() {
  const { uploads } = useApp();
  const values = [...uploads.collections.values()].filter(
    (value) => value.running || value.creating || value.pending,
  );
  if (!values.length) return null;
  return (
    <div id="active-uploads" className="active-uploads">
      {values.map((value) => {
        const items = value.items.filter((item) => !item.removedAt);
        const total = items.reduce((sum, item) => sum + item.size, 0);
        const committed = items.reduce(
          (sum, item) => sum + (item.stored?.offset ?? 0),
          0,
        );
        return (
          <div key={value.key} className="upload-indicator" role="status">
            <LoaderCircle aria-hidden="true" className="animate-spin" />
            <div>
              <a href={value.share ? `#share/${value.key}` : "#new"}>
                {value.creating
                  ? "Creating private draft…"
                  : value.publishing
                    ? "Preparing share…"
                    : !value.running
                      ? "Saving draft…"
                      : (value.share?.title ?? "Uploading files")}
              </a>
              {value.running && (
                <>
                  <span className="fine muted">
                    {bytes(committed)} of {bytes(total)}
                  </span>
                  <Progress
                    aria-label="Upload progress"
                    value={total ? (committed / total) * 100 : 0}
                  />
                </>
              )}
            </div>
            {value.running && (
              <IconButton
                label={`Pause upload${value.share ? ` for ${value.share.title}` : ""}`}
                onClick={() => uploads.pause(value.key)}
              >
                <Pause aria-hidden="true" />
              </IconButton>
            )}
          </div>
        );
      })}
    </div>
  );
}
