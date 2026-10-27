import { createContext, useContext } from "react";
import type { Session, User } from "@/types/api";
import type { UploadManager } from "@/transfers/upload-manager";
export interface AppContextValue {
  user: User | null | undefined;
  csrf: string;
  notify: (message: string, error?: boolean, href?: string) => void;
  signedIn: (session: Session) => void;
  signedOut: () => void;
  logout: () => Promise<void>;
  uploads: UploadManager;
  navigate: (hash: string) => void;
}
export const AppContext = createContext<AppContextValue | null>(null);
export function useApp() {
  const app = useContext(AppContext);
  if (!app) throw new Error("Missing application provider");
  return app;
}
