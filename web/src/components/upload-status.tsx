import { Button } from "./ui/button";
import { Progress } from "./ui/progress";
import { useApp } from "@/app/context";
import { bytes } from "@/lib/format";

export function UploadStatus() {
  const { progress, pauseUpload } = useApp();
  return (
    <div id="upload-status" aria-live="polite">
      {progress && (
        <div className="transfer-progress">
          <strong id="upload-label">{progress.label}</strong>
          <Progress
            id="progress"
            aria-label="Upload progress"
            value={
              progress.total ? (progress.committed / progress.total) * 100 : 0
            }
          />
          <div>
            <span id="upload-detail" className="fine muted">
              {bytes(progress.committed)} of {bytes(progress.total)}
            </span>
            <Button
              type="button"
              variant="ghost"
              data-action="pause"
              onClick={pauseUpload}
            >
              Pause upload
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
