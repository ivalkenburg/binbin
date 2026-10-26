import { api } from "@/lib/api";
import { bytes } from "@/lib/format";
import type { Share, StoredFile } from "@/types/api";
const CHUNK = 8 * 1024 * 1024;
export interface UploadProgress {
  label: string;
  committed: number;
  total: number;
}
async function verifyPrefix(
  file: File,
  stored: StoredFile,
  signal: AbortSignal,
) {
  signal.throwIfAborted();
  if (!stored.offset) return;
  const proofs = await api<string[]>(`/api/files/${stored.id}/proofs`, {
    signal,
  });
  for (let i = 0; i < proofs.length; i++) {
    signal.throwIfAborted();
    const data = await file
      .slice(i * CHUNK, Math.min(stored.offset, (i + 1) * CHUNK))
      .arrayBuffer();
    const hash = Array.from(
      new Uint8Array(await crypto.subtle.digest("SHA-256", data)),
      (b) => b.toString(16).padStart(2, "0"),
    ).join("");
    signal.throwIfAborted();
    if (hash !== proofs[i])
      throw new Error(
        `${file.name} has changed. Start a new draft with the changed file.`,
      );
  }
}
export async function uploadFiles(
  draft: Share,
  files: File[],
  signal: AbortSignal,
  progress: (value: UploadProgress) => void,
) {
  if (files.length !== draft.files.length)
    throw new Error("Choose the complete original file set.");
  let committed = draft.files.reduce((n, f) => n + f.offset, 0);
  for (const stored of draft.files) {
    const file = files.find(
      (f) => f.name === stored.name && f.size === stored.size,
    );
    if (!file)
      throw new Error(
        `Choose the original ${stored.name} (${bytes(stored.size)}).`,
      );
    await verifyPrefix(file, stored, signal);
    let offset = stored.offset;
    while (offset < file.size) {
      signal.throwIfAborted();
      progress({
        label: `Uploading ${stored.name}`,
        committed,
        total: draft.original_size,
      });
      const result = await api<{ offset: number }>(
        `/api/files/${stored.id}?offset=${offset}`,
        { method: "PATCH", body: file.slice(offset, offset + CHUNK), signal },
      );
      signal.throwIfAborted();
      if (result.offset <= offset || result.offset > file.size)
        throw new Error("Invalid upload offset. Refresh the draft and resume.");
      committed += result.offset - offset;
      offset = result.offset;
      progress({
        label: `Uploading ${stored.name}`,
        committed,
        total: draft.original_size,
      });
    }
  }
}
