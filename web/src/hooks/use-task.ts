import { useEffect, useRef, useState } from "react";
import { useApp } from "@/app/context";
import { errorMessage } from "@/lib/api";
export function useTask() {
  const operation = useRef<AbortController | null>(null);
  const [pending, setPending] = useState(false);
  const { notify } = useApp();
  useEffect(() => () => operation.current?.abort(), []);
  async function run(
    task: (signal: AbortSignal) => Promise<void>,
    onError?: (message: string) => void,
  ) {
    if (operation.current) return;
    const controller = new AbortController();
    operation.current = controller;
    setPending(true);
    try {
      await task(controller.signal);
    } catch (error) {
      if (!controller.signal.aborted) {
        if (onError) onError(errorMessage(error));
        else notify(errorMessage(error), true);
      }
    } finally {
      if (operation.current === controller) {
        operation.current = null;
        setPending(false);
      }
    }
  }
  return { pending, run };
}
