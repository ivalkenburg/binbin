import { DownloadButton, FileOpenButton } from "./download-controls";
import { FileTypeIcon } from "./file-preview";
import { bytes, fileCount } from "@/lib/format";
import type { PublicShare, Share } from "@/types/api";

export function FileList({
  share,
  management = false,
  password,
  downloadable = true,
}: {
  share: Share | PublicShare;
  management?: boolean;
  password?: string | null;
  downloadable?: boolean;
}) {
  const total = share.files.reduce((n, file) => n + file.size, 0);
  const single = share.files.length === 1 && !management;
  const transfer = { share, management, password };
  return (
    <div className="file-list">
      {!single && (
        <div className="file-list-head">
          <span>
            <strong>{fileCount(share.files.length)}</strong>
            <span className="muted"> · {bytes(total)}</span>
          </span>
          {downloadable && share.zip_hash && (
            <DownloadButton {...transfer} resource="zip" primary={!management}>
              Download all as ZIP · {bytes(share.zip_size)}
            </DownloadButton>
          )}
        </div>
      )}
      {share.files.map((file) => (
        <div className="file-row" key={file.id}>
          <FileTypeIcon name={file.name} />
          {downloadable ? (
            <FileOpenButton {...transfer} resource={file.id}>
              {file.name}
            </FileOpenButton>
          ) : (
            <strong className="file-name" title={file.name}>
              {file.name}
            </strong>
          )}
          <span className="file-size">{bytes(file.size)}</span>
          {downloadable && (
            <DownloadButton
              {...transfer}
              resource={file.id}
              iconOnly
              title={`Download ${file.name}`}
            />
          )}
        </div>
      ))}
    </div>
  );
}
