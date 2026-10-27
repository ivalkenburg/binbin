import { api } from "@/lib/api";
import type { StoredFile } from "@/types/api";
const CHUNK = 8 * 1024 * 1024;
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
        `${file.name} differs from the uploaded prefix. Select the original file, or remove this item and add the changed file.`,
      );
  }
}
export async function uploadFile(
  stored: StoredFile,
  file: File,
  signal: AbortSignal,
  progress: (value: StoredFile) => void,
) {
  await verifyPrefix(file, stored, signal);
  let offset = stored.offset;
  while (offset < file.size) {
    signal.throwIfAborted();
    const result = await api<StoredFile>(
      `/api/files/${stored.id}?offset=${offset}`,
      { method: "PATCH", body: file.slice(offset, offset + CHUNK), signal },
    );
    signal.throwIfAborted();
    if (result.offset <= offset || result.offset > file.size)
      throw new Error("Invalid upload offset. Refresh the draft and resume.");
    offset = result.offset;
    progress(result);
  }
}
