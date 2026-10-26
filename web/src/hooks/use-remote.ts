import { useEffect, useState } from "react";
import { api } from "@/lib/api";
export function useRemote<T>(path: string, body?: unknown) {
  const [data, setData] = useState<T>();
  const [error, setError] = useState<unknown>();
  const [revision, setRevision] = useState(0);
  const serialized = body === undefined ? undefined : JSON.stringify(body);
  useEffect(() => {
    const controller = new AbortController();
    setError(undefined);
    api<T>(path, {
      method: serialized === undefined ? "GET" : "POST",
      body: serialized === undefined ? undefined : JSON.parse(serialized),
      signal: controller.signal,
    })
      .then((value) => {
        if (!controller.signal.aborted) setData(value);
      })
      .catch((error) => {
        if (!controller.signal.aborted) {
          setError(error);
        }
      });
    return () => controller.abort();
  }, [path, serialized, revision]);
  return { data, error, reload: () => setRevision((n) => n + 1) };
}
