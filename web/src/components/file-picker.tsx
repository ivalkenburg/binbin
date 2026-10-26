import { useState } from "react";
import { File as FileIcon, Upload } from "lucide-react";
import { Label } from "./ui/label";
import { bytes, fileCount } from "@/lib/format";

export function FilePicker({
  files,
  onChange,
  disabled = false,
  limits,
}: {
  files: File[];
  onChange: (files: File[]) => void;
  disabled?: boolean;
  limits?: { file_limit: number; file_count: number };
}) {
  const [dragging, setDragging] = useState(false);
  return (
    <>
      <Label
        id="drop-zone"
        className={`drop-zone${dragging && !disabled ? " dragging" : ""}`}
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragging(false);
          if (!disabled) onChange(Array.from(e.dataTransfer.files));
        }}
      >
        <Upload aria-hidden="true" />
        <strong>Drop files here, or choose files</strong>
        <span>Select the complete file set for this share.</span>
        {limits && (
          <span className="fine">
            Up to {limits.file_count} files · {bytes(limits.file_limit)} per
            file
          </span>
        )}
        <input
          id="files"
          type="file"
          multiple
          aria-label="Choose files"
          disabled={disabled}
          onChange={(e) => onChange(Array.from(e.target.files || []))}
        />
      </Label>
      <div id="selected-files">
        {files.map((file, i) => (
          <div className="selected-file" key={i}>
            <FileIcon aria-hidden="true" />
            <span title={file.name}>{file.name}</span>
            <small>{bytes(file.size)}</small>
          </div>
        ))}
        {files.length > 0 && (
          <p className="fine muted">
            {fileCount(files.length)} ·{" "}
            {bytes(files.reduce((n, f) => n + f.size, 0))} total
          </p>
        )}
      </div>
    </>
  );
}
