import { createContext, useContext } from "react";
import type { Session, Share, User } from "@/types/api";
import type { UploadProgress } from "@/transfers/upload";
export interface AppContextValue {
  user: User | null | undefined;
  csrf: string;
  notice: { message: string; error: boolean } | null;
  notify: (message: string, error?: boolean) => void;
  signedIn: (session: Session) => void;
  signedOut: () => void;
  logout: () => Promise<void>;
  busy: boolean;
  progress: UploadProgress | null;
  pauseUpload: () => void;
  upload: (
    load: (signal: AbortSignal) => Promise<Share>,
    files: File[],
  ) => Promise<void>;
}
export const AppContext = createContext<AppContextValue | null>(null);
export function useApp() {
  const app = useContext(AppContext);
  if (!app) throw new Error("Missing application provider");
  return app;
}
