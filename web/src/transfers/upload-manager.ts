import { api, errorMessage } from "@/lib/api";
import { bytes } from "@/lib/format";
import type { Limits, NewShare, Share, StoredFile } from "@/types/api";
import { uploadFile } from "./upload";

export type FileState =
  "queued" | "checking" | "uploading" | "uploaded" | "needs-file" | "failed";
export interface UploadItem {
  key: string;
  name: string;
  size: number;
  local?: File;
  stored?: StoredFile;
  state: FileState;
  error?: string;
  removedAt?: number;
  resuming?: boolean;
}
export interface UploadCollection {
  key: string;
  share?: Share;
  items: UploadItem[];
  running: boolean;
  paused: boolean;
  creating: boolean;
  publishing: boolean;
  pending: number;
  error?: string;
  run: number;
  controller?: AbortController;
  mutations: Promise<void>;
  details?: Record<string, string>;
}
function collection(key: string): UploadCollection {
  return {
    key,
    items: [],
    running: false,
    paused: false,
    creating: false,
    publishing: false,
    pending: 0,
    run: 0,
    mutations: Promise.resolve(),
  };
}
function fromStored(stored: StoredFile): UploadItem {
  return {
    key: stored.id,
    name: stored.name,
    size: stored.size,
    stored,
    resuming: stored.offset < stored.size,
    state: stored.offset === stored.size ? "uploaded" : "needs-file",
  };
}
function uniqueName(name: string, used: Set<string>) {
  if (!used.has(name.toLowerCase())) return name;
  const dot = name.lastIndexOf(".");
  const extension = dot > 0 ? name.slice(dot) : "";
  const base = dot > 0 ? name.slice(0, dot) : name;
  let index = 2;
  let result: string;
  do {
    const suffix = ` (${index++})${extension}`;
    const stem = Array.from(base);
    while (
      new TextEncoder().encode(stem.join("") + suffix).length > 240 &&
      stem.length
    )
      stem.pop();
    result = stem.join("") + suffix;
  } while (used.has(result.toLowerCase()));
  return result;
}
export function fileError(item: UploadItem, limits?: Limits) {
  if (
    !item.name ||
    new TextEncoder().encode(item.name).length > 240 ||
    Array.from(item.name).some(
      (char) =>
        char.charCodeAt(0) < 32 ||
        (char.charCodeAt(0) >= 127 && char.charCodeAt(0) <= 159) ||
        "/\\:".includes(char),
    ) ||
    item.name === "." ||
    item.name === ".."
  )
    return "Rename this file to a safe name of at most 240 bytes.";
  if (limits && item.size > limits.settings.file_limit)
    return `Exceeds the ${bytes(limits.settings.file_limit)} file limit.`;
}
export function collectionErrors(value: UploadCollection, limits?: Limits) {
  const items = value.items.filter((item) => !item.removedAt);
  const errors = items
    .map((item) => fileError(item, limits))
    .filter((value): value is string => !!value);
  if (!limits) return [...new Set(errors)];
  if (items.length > limits.settings.file_count)
    errors.push(`Choose at most ${limits.settings.file_count} files.`);
  const size = items.reduce((sum, item) => sum + item.size, 0);
  if (size > limits.settings.share_limit)
    errors.push(
      `Exceeds the ${bytes(limits.settings.share_limit)} share limit.`,
    );
  const held = value.items
    .filter((item) => item.removedAt && item.stored)
    .reduce((sum, item) => sum + item.size, 0);
  const required =
    size + held + (items.length > 1 ? size + items.length * 1024 + 1024 : 0);
  if (
    required >
    limits.settings.storage_limit - limits.used + (value.share?.reserved ?? 0)
  )
    errors.push("Not enough available storage for these files and their ZIP.");
  return [...new Set(errors)];
}

/** Owns local File handles and requests across route changes; reset at session end. */
export class UploadManager {
  onNotice?: (message: string, error?: boolean, href?: string) => void;
  collections = new Map<string, UploadCollection>([["new", collection("new")]]);
  limits?: Limits;
  private revision = 0;
  private listeners = new Set<() => void>();
  private lifetime = new AbortController();
  private timers = new Map<UploadItem, ReturnType<typeof setTimeout>>();
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  snapshot = () => this.revision;
  private emit() {
    this.revision++;
    this.listeners.forEach((listener) => listener());
  }
  get busy() {
    return [...this.collections.values()].some(
      (value) => value.running || value.creating || value.pending > 0,
    );
  }
  get(key: string) {
    return this.collections.get(key);
  }
  reset() {
    this.lifetime.abort();
    this.collections.forEach((value) => value.controller?.abort());
    this.timers.forEach((timer) => clearTimeout(timer));
    this.timers.clear();
    this.lifetime = new AbortController();
    this.collections = new Map([["new", collection("new")]]);
    this.limits = undefined;
    this.emit();
  }
  setLimits(limits: Limits) {
    this.limits = limits;
    this.emit();
  }
  saveDetails(details: Record<string, string>) {
    this.get("new")!.details = details;
  }
  private async refreshLimits(signal: AbortSignal) {
    try {
      this.limits = await api<Limits>("/api/settings", { signal });
    } catch {
      /* Keep the last limits; the server still validates every edit. */
    }
  }
  hydrate(share: Share) {
    const existing = this.get(share.id);
    if (
      existing &&
      (existing.running ||
        existing.pending ||
        existing.publishing ||
        share.version < (existing.share?.version ?? 0) ||
        (share.state === "draft" && existing.share?.state !== "draft"))
    )
      return;
    const changed =
      existing?.share &&
      (existing.share.version !== share.version ||
        existing.share.reserved !== share.reserved);
    const value = existing ?? collection(share.id);
    this.collections.set(share.id, value);
    this.merge(value, share, true);
    this.emit();
    if (changed) {
      const signal = this.lifetime.signal;
      void this.refreshLimits(signal).then(() => {
        if (!signal.aborted) this.emit();
      });
    }
  }
  private merge(value: UploadCollection, share: Share, reconcile = false) {
    if (this.limits && value.share)
      this.limits = {
        ...this.limits,
        used: Math.max(
          0,
          this.limits.used + share.reserved - value.share.reserved,
        ),
      };
    value.share = share;
    const old = value.items;
    value.items = share.files.map((stored) => {
      const item =
        old.find((item) => item.stored?.id === stored.id) ??
        old.find(
          (item) =>
            !item.stored &&
            item.name === stored.name &&
            item.size === stored.size,
        ) ??
        fromStored(stored);
      const offset = Math.max(item.stored?.offset ?? 0, stored.offset);
      item.stored = { ...stored, offset };
      if (reconcile) item.removedAt = undefined;
      if (offset === stored.size) {
        item.state = "uploaded";
        item.error = undefined;
      }
      return item;
    });
    for (const removed of share.removed_files ?? []) {
      const item =
        old.find((item) => item.stored?.id === removed.file.id) ??
        fromStored(removed.file);
      item.stored = removed.file;
      const relativeTime =
        Date.now() - Math.max(0, share.server_time - removed.removed_at) * 1000;
      item.removedAt = Math.min(item.removedAt ?? relativeTime, relativeTime);
      value.items.push(item);
      this.expireRemoval(value, item);
    }
    value.items.push(
      ...old.filter((item) => !item.stored && !value.items.includes(item)),
    );
  }
  private enqueue(
    value: UploadCollection,
    action: (signal: AbortSignal) => Promise<void>,
  ) {
    value.pending++;
    this.emit();
    const signal = this.lifetime.signal;
    const result = value.mutations.then(async () => {
      signal.throwIfAborted();
      await action(signal);
    });
    value.mutations = result
      .catch(async (error) => {
        if (signal.aborted) return;
        value.error = errorMessage(error);
        // A mutation can commit even when its response is lost. Reconcile IDs
        // and removals before allowing a retry, preserving local File handles.
        if (value.share) {
          try {
            const share = await api<Share>(`/api/shares/${value.key}`, {
              signal,
            });
            if (!signal.aborted) this.merge(value, share, true);
          } catch {
            /* Keep the actionable original error if refresh also fails. */
          }
        }
      })
      .finally(() => {
        value.pending--;
        if (!signal.aborted) this.emit();
      });
    return result;
  }
  private async register(value: UploadCollection, signal: AbortSignal) {
    const errors = collectionErrors(value, this.limits);
    if (errors.length) throw new Error(errors[0]);
    const additions = value.items.filter(
      (item) => !item.removedAt && !item.stored,
    );
    if (!additions.length || !value.share) return;
    const share = await api<Share>(`/api/shares/${value.key}/files`, {
      method: "PATCH",
      signal,
      body: {
        add: additions.map((item) => ({ name: item.name, size: item.size })),
      },
    });
    signal.throwIfAborted();
    this.merge(value, share);
    await this.refreshLimits(signal);
    value.error = undefined;
    this.emit();
  }
  add(key: string, files: File[]) {
    const value = this.get(key);
    if (!value || value.creating || !files.length) return;
    const used = new Set(value.items.map((item) => item.name.toLowerCase()));
    let renamed = 0;
    for (const local of files) {
      const missing = value.items.find(
        (item) =>
          !item.removedAt &&
          !item.local &&
          item.state !== "uploaded" &&
          item.name === local.name &&
          item.size === local.size,
      );
      if (missing) {
        missing.local = local;
        missing.state = "queued";
        missing.error = undefined;
        continue;
      }
      const name = uniqueName(local.name, used);
      if (name !== local.name) renamed++;
      used.add(name.toLowerCase());
      value.items.push({
        key: crypto.randomUUID(),
        name,
        size: local.size,
        local,
        state: "queued",
      });
    }
    value.error = undefined;
    this.emit();
    if (value.share) {
      const joining = value.running;
      void this.enqueue(value, (signal) => this.register(value, signal))
        .then(() => {
          if (joining) void this.start(key);
        })
        .catch(() => {});
    }
    return renamed;
  }
  attach(key: string, itemKey: string, local: File) {
    const value = this.get(key);
    const item = value?.items.find((item) => item.key === itemKey);
    if (!value || !item || item.removedAt) return;
    if (local.size !== item.size) {
      item.error = `Choose the original file (${bytes(item.size)}).`;
      item.state = "failed";
    } else {
      item.local = local;
      item.state = "queued";
      item.error = undefined;
    }
    this.emit();
    if (value.running) void this.start(key);
  }
  async create(body: Omit<NewShare, "files">, onCreated: (id: string) => void) {
    const value = this.get("new")!;
    if (value.creating || !value.items.some((item) => !item.removedAt)) return;
    const errors = collectionErrors(value, this.limits);
    if (errors.length) {
      value.error = errors[0];
      this.emit();
      return;
    }
    value.creating = true;
    value.error = undefined;
    this.emit();
    const signal = this.lifetime.signal;
    try {
      const share = await api<Share>("/api/shares", {
        method: "POST",
        signal,
        body: {
          ...body,
          files: value.items
            .filter((item) => !item.removedAt)
            .map((item) => ({ name: item.name, size: item.size })),
        },
      });
      signal.throwIfAborted();
      value.items = value.items.filter((item) => !item.removedAt);
      value.key = share.id;
      value.creating = false;
      value.details = undefined;
      this.merge(value, share);
      this.collections.set(share.id, value);
      this.collections.set("new", collection("new"));
      await this.refreshLimits(signal);
      signal.throwIfAborted();
      this.emit();
      onCreated(share.id);
      void this.start(share.id);
    } catch (error) {
      if (!signal.aborted) {
        value.creating = false;
        value.error = errorMessage(error);
        this.onNotice?.(value.error, true, "#new");
        this.emit();
      }
    }
  }
  async start(key: string) {
    const value = this.get(key);
    if (!value?.share || value.running) return;
    const run = ++value.run;
    const lifetime = this.lifetime.signal;
    value.running = true;
    value.paused = false;
    value.error = undefined;
    this.emit();
    try {
      await this.enqueue(value, (signal) => this.register(value, signal));
      while (!lifetime.aborted && value.run === run) {
        await value.mutations;
        if (value.run !== run || lifetime.aborted) break;
        const item = value.items.find(
          (item) =>
            !item.removedAt &&
            item.local &&
            item.stored &&
            item.state === "queued",
        );
        if (!item?.stored || !item.local) break;
        const controller = new AbortController();
        value.controller = controller;
        const signal = AbortSignal.any([lifetime, controller.signal]);
        item.state = item.stored.offset ? "checking" : "uploading";
        this.emit();
        try {
          // Fetch the durable offset: an interrupted request may have committed its chunk.
          const current = await api<Share>(`/api/shares/${key}`, { signal });
          if (value.run !== run || signal.aborted) break;
          const stored = current.files.find(
            (file) => file.id === item.stored?.id,
          );
          if (!stored) throw new Error("This file is no longer in the draft.");
          item.stored = stored;
          await uploadFile(stored, item.local, signal, (file) => {
            if (value.run !== run || signal.aborted) return;
            item.stored = file;
            item.state = "uploading";
            this.emit();
          });
          if (value.run === run && !signal.aborted && !item.removedAt) {
            item.state = "uploaded";
            item.error = undefined;
          }
        } catch (error) {
          if (value.run !== run || lifetime.aborted) break;
          if (signal.aborted) {
            if (!item.removedAt) item.state = "queued";
          } else {
            item.state = "failed";
            item.error = errorMessage(error);
          }
        }
        this.emit();
      }
    } catch (error) {
      if (value.run === run && !lifetime.aborted)
        value.error = errorMessage(error);
    } finally {
      if (value.run === run && !lifetime.aborted) {
        value.running = false;
        value.controller = undefined;
        const items = value.items.filter((item) => !item.removedAt);
        if (value.error || items.some((item) => item.state === "failed"))
          this.onNotice?.(
            "Some files could not upload. Review the draft to retry or remove them.",
            true,
            `#share/${key}`,
          );
        else if (
          items.length &&
          items.every((item) => item.state === "uploaded")
        )
          this.onNotice?.(
            "Upload complete. Review your files and publish when ready.",
            false,
            `#share/${key}`,
          );
        this.emit();
      }
    }
  }
  pause(key: string) {
    const value = this.get(key);
    if (!value) return;
    value.run++;
    value.controller?.abort();
    value.items.forEach((item) => {
      if (item.state === "uploading" || item.state === "checking")
        item.state = "queued";
    });
    value.running = false;
    value.paused = true;
    this.emit();
  }
  retry(key: string, itemKey: string) {
    const value = this.get(key);
    const item = value?.items.find((item) => item.key === itemKey);
    if (!item || !value) return;
    item.state = item.local ? "queued" : "needs-file";
    item.error = undefined;
    this.emit();
    if (item.local) void this.start(key);
  }
  private expireRemoval(value: UploadCollection, item: UploadItem) {
    clearTimeout(this.timers.get(item));
    const removedAt = item.removedAt;
    const timer = setTimeout(
      () => {
        this.timers.delete(item);
        if (item.removedAt !== removedAt) return;
        value.items = value.items.filter((candidate) => candidate !== item);
        this.emit();
      },
      Math.max(0, (removedAt ?? 0) + 8000 - Date.now()),
    );
    this.timers.set(item, timer);
  }
  async remove(key: string, itemKeys: string[]) {
    const value = this.get(key);
    if (!value || value.creating) return;
    const items = value.items.filter(
      (item) => itemKeys.includes(item.key) && !item.removedAt,
    );
    items.forEach((item) => {
      item.removedAt = Date.now();
      if (item.state === "uploading" || item.state === "checking") {
        value.controller?.abort();
        item.state = "queued";
      }
    });
    this.emit();
    if (!value.share) {
      items.forEach((item) => this.expireRemoval(value, item));
      return;
    }
    try {
      await this.enqueue(value, async (signal) => {
        const ids = items.flatMap((item) =>
          item.stored ? [item.stored.id] : [],
        );
        if (ids.length) {
          const share = await api<Share>(`/api/shares/${key}/files`, {
            method: "PATCH",
            signal,
            body: { remove: ids },
          });
          this.merge(value, share);
        }
        items.forEach((item) => {
          if (item.removedAt) this.expireRemoval(value, item);
        });
        value.error = undefined;
      });
    } catch {
      items.forEach((item) => {
        item.removedAt = undefined;
      });
      this.emit();
    }
  }
  async undo(key: string, itemKey: string) {
    const value = this.get(key);
    const item = value?.items.find(
      (item) => item.key === itemKey && item.removedAt,
    );
    if (!value || !item) return;
    if (!item.stored) {
      item.removedAt = undefined;
      this.emit();
      return;
    }
    try {
      await this.enqueue(value, async (signal) => {
        const share = await api<Share>(`/api/shares/${key}/files`, {
          method: "PATCH",
          signal,
          body: { restore: [item.stored!.id] },
        });
        item.removedAt = undefined;
        this.merge(value, share);
        item.state =
          item.stored?.offset === item.size
            ? "uploaded"
            : item.local
              ? "queued"
              : "needs-file";
        value.error = undefined;
      });
    } catch {
      /* enqueue exposes the error next to the collection */
    }
  }
  async publish(key: string) {
    const value = this.get(key);
    if (!value || value.running || value.pending) return;
    value.publishing = true;
    this.emit();
    try {
      await this.enqueue(value, async (signal) => {
        if (
          value.items.some(
            (item) => !item.removedAt && item.state !== "uploaded",
          )
        )
          throw new Error(
            "Upload or remove every unfinished file before publishing.",
          );
        const share = await api<Share>(`/api/shares/${key}/publish`, {
          method: "POST",
          signal,
        });
        this.merge(value, share);
      });
    } finally {
      value.publishing = false;
      this.emit();
    }
  }
}
