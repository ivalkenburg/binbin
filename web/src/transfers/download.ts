import { api, ApiError } from "@/lib/api";
interface SavedTransfer {
  key: string;
  token?: string;
  expires?: number;
}
interface PendingTransfer {
  signal: AbortSignal;
  promise: Promise<string>;
}
const pendingPreparations = new Map<string, PendingTransfer>();
const pendingDownloads = new Map<string, PendingTransfer>();
const memory = new Map<string, SavedTransfer>();
function read(key: string): SavedTransfer | null {
  if (memory.has(key)) return memory.get(key)!;
  try {
    const saved = JSON.parse(sessionStorage.getItem(key) || "null");
    return saved && typeof saved.key === "string" ? saved : null;
  } catch {
    return null;
  }
}
function save(key: string, value: SavedTransfer) {
  memory.set(key, value);
  try {
    sessionStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* In-memory retries remain available. */
  }
}
export interface DownloadRequest {
  shareId: string;
  resource: string;
  management: boolean;
  scope: string;
  password?: string | null;
  signal: AbortSignal;
}
export function download(request: DownloadRequest) {
  return coalesce(pendingDownloads, request, async () => {
    const href = await prepareDownload(request);
    triggerDownload(href, request.signal);
    return href;
  });
}

export async function openFile(request: DownloadRequest) {
  // Reserve the tab during the click so asynchronous issuance is not blocked
  // as an unsolicited popup. Do not charge a transfer if the tab is blocked.
  const tab = window.open("about:blank", "_blank");
  if (!tab)
    throw new Error("Allow a new tab to open this file, then try again.");
  tab.opener = null;
  try {
    const href = await prepareDownload(request);
    request.signal.throwIfAborted();
    tab.location.replace(`${href}?inline=true`);
    return href;
  } catch (error) {
    tab.close();
    throw error;
  }
}

function prepareDownload(request: DownloadRequest) {
  return coalesce(pendingPreparations, request, (key) => start(request, key));
}

function coalesce(
  operations: Map<string, PendingTransfer>,
  request: DownloadRequest,
  run: (key: string) => Promise<string>,
): Promise<string> {
  if (request.signal.aborted) return Promise.reject(request.signal.reason);
  const key = `binbin:${request.shareId}:${request.resource}:${request.scope}`;
  const existing = operations.get(key);
  if (existing)
    return existing.promise
      .then((href) => {
        request.signal.throwIfAborted();
        return href;
      })
      .catch((error) => {
        // A cancelled initiator must not cancel another control's live request.
        if (existing.signal.aborted && !request.signal.aborted)
          return coalesce(operations, request, run);
        throw error;
      });
  const promise = run(key).finally(() => operations.delete(key));
  operations.set(key, { signal: request.signal, promise });
  return promise;
}
async function start(
  { shareId, resource, management, password, signal }: DownloadRequest,
  storageKey: string,
) {
  let saved = read(storageKey);
  let validated = false;
  if (saved?.token) {
    const response = await fetch(`/d/${encodeURIComponent(saved.token)}`, {
      method: "HEAD",
      signal,
    });
    if (!response.ok) {
      if (response.status !== 401 && response.status !== 404)
        throw new Error(
          `Cannot start download (${response.status}). Try again.`,
        );
      saved = null;
    } else validated = true;
  }
  if (!saved?.token) {
    const recoveringIssuance = !!saved;
    saved ??= { key: crypto.randomUUID() };
    save(storageKey, saved);
    const issue = (key: string) =>
      api<{ token: string; expires: number }>(
        `/api/${management ? "shares" : "public"}/${shareId}/transfers`,
        { method: "POST", body: { resource, key, password }, signal },
      );
    let issued;
    try {
      issued = await issue(saved.key);
    } catch (error) {
      if (
        !recoveringIssuance ||
        !(error instanceof ApiError) ||
        error.status !== 404
      )
        throw error;
      // A saved key without a capability can outlive its failed issuance attempt.
      saved = { key: crypto.randomUUID() };
      save(storageKey, saved);
      issued = await issue(saved.key);
    }
    saved = { ...issued, key: saved.key };
    save(storageKey, saved);
  }
  const href = `/d/${encodeURIComponent(saved.token!)}`;
  if (!validated) {
    const response = await fetch(href, { method: "HEAD", signal });
    if (!response.ok)
      throw new Error(
        "This download is no longer available. Reopen the share and try again.",
      );
  }
  return href;
}

export interface RecoveredDownload {
  resource: string;
  token: string;
  label: string;
}

export async function recoverPublicDownloads(
  shareId: string,
  signal: AbortSignal,
) {
  const prefix = `binbin:${shareId}:`;
  const keys = new Set(memory.keys());
  try {
    for (let index = 0; index < sessionStorage.length; index++) {
      const key = sessionStorage.key(index);
      if (key) keys.add(key);
    }
  } catch {
    /* Storage can be unavailable; current-tab capabilities still work. */
  }
  const recovered: RecoveredDownload[] = [];
  for (const key of keys) {
    if (!key.startsWith(prefix) || !key.endsWith(":public")) continue;
    const saved = read(key);
    if (!saved?.token) continue;
    const response = await fetch(`/d/${encodeURIComponent(saved.token)}`, {
      method: "HEAD",
      signal,
    });
    if (response.status === 401 || response.status === 404) continue;
    if (!response.ok)
      throw new Error(
        "Couldn’t check previous downloads. Check your connection and try again.",
      );
    const resource = key.slice(prefix.length, -":public".length);
    const disposition = response.headers.get("content-disposition") || "";
    const encoded = disposition.match(/filename\*=UTF-8''([^;]+)/i)?.[1];
    let label =
      resource === "zip" ? "All files (ZIP)" : "Previously started file";
    if (encoded) {
      try {
        label = decodeURIComponent(encoded);
      } catch {
        /* Retain the generic label for malformed headers. */
      }
    }
    recovered.push({ resource, token: saved.token, label });
  }
  return recovered;
}

export async function retryRecoveredDownload(
  token: string,
  signal: AbortSignal,
) {
  const href = `/d/${encodeURIComponent(token)}`;
  const response = await fetch(href, { method: "HEAD", signal });
  if (!response.ok) throw new Error("This transfer is no longer available.");
  triggerDownload(href, signal);
}

function triggerDownload(href: string, signal: AbortSignal) {
  signal.throwIfAborted();
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = "";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
}
