import { LinkButton } from "@/components/link-button";
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { Files, FilePlus2, Settings, Shield, LogOut, X } from "lucide-react";
import { AppContext } from "./context";
import {
  api,
  ApiError,
  errorMessage,
  setCsrf,
  setSessionExpiredHandler,
} from "@/lib/api";
import { UploadManager } from "@/transfers/upload-manager";
import { UploadStatus } from "@/components/upload-status";
import { ThemeControl } from "@/components/theme-control";
import { TooltipProvider } from "@/components/ui/tooltip";
import { IconButton } from "@/components/icon-button";
import { useTheme } from "@/hooks/use-theme";
import { Alert, AlertDescription, AlertAction } from "@/components/ui/alert";
import { LoginPage } from "@/pages/login-page";
import { DashboardPage } from "@/pages/dashboard-page";
import { NewSharePage } from "@/pages/new-share-page";
import { SharePage } from "@/pages/share-page";
import { SettingsPage } from "@/pages/settings-page";
import { AdminPage } from "@/pages/admin-page";
import { PublicPage } from "@/pages/public-page";
import type { Session } from "@/types/api";
const recipient = location.pathname.startsWith("/s/");
export function App() {
  const [session, setSession] = useState<Session | null | undefined>(
    recipient ? null : undefined,
  );
  const [route, setRoute] = useState(location.hash.slice(1) || "shares");
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState<{
    message: string;
    error: boolean;
    href?: string;
  } | null>(null);
  const [uploads] = useState(() => new UploadManager());
  useSyncExternalStore(uploads.subscribe, uploads.snapshot);
  const { theme, setTheme } = useTheme();
  const notify = useCallback(
    (message: string, error = false, href?: string) =>
      setNotice(message ? { message, error, href } : null),
    [],
  );
  useEffect(() => {
    uploads.onNotice = notify;
    return () => {
      uploads.onNotice = undefined;
    };
  }, [notify, uploads]);
  const signedOut = useCallback(() => {
    uploads.reset();
    setCsrf("");
    setSession(null);
  }, [uploads]);
  useEffect(() => {
    setSessionExpiredHandler(signedOut);
    return () => setSessionExpiredHandler();
  }, [signedOut]);
  const signedIn = useCallback((value: Session) => {
    setCsrf(value.csrf);
    setSession(value);
    setNotice(null);
  }, []);
  useEffect(() => {
    if (recipient) return;
    const abort = new AbortController();
    api<Session>("/api/me", { signal: abort.signal })
      .then(signedIn)
      .catch(() => {
        if (!abort.signal.aborted) signedOut();
      });
    return () => abort.abort();
  }, [signedIn, signedOut]);
  useEffect(() => {
    const changed = () => {
      setRoute(location.hash.slice(1) || "shares");
      setNotice(null);
    };
    const unloading = (event: BeforeUnloadEvent) => {
      if (uploads.busy) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("hashchange", changed);
    window.addEventListener("beforeunload", unloading);
    return () => {
      window.removeEventListener("hashchange", changed);
      window.removeEventListener("beforeunload", unloading);
    };
  }, [uploads]);
  useEffect(() => {
    document.body.className = recipient
      ? "recipient"
      : session
        ? ""
        : "signed-out";
    document.title = recipient
      ? "Shared files · Binbin"
      : session
        ? "Binbin"
        : "Sign in · Binbin";
  }, [session]);
  const navigate = useCallback((hash: string) => {
    history.replaceState(
      null,
      "",
      `${location.pathname}${location.search}#${hash}`,
    );
    setRoute(hash);
    setRevision((n) => n + 1);
  }, []);
  const logout = useCallback(async () => {
    uploads.collections.forEach((value) => {
      if (value.running) uploads.pause(value.key);
    });
    try {
      await api("/api/logout", { method: "POST" });
      signedOut();
    } catch (error) {
      if (!(error instanceof ApiError && error.status === 401))
        notify(errorMessage(error), true);
    }
  }, [notify, signedOut, uploads]);
  const [page, id] = route.split("/");
  const authenticated = !!session && !recipient;
  const content = recipient ? (
    <PublicPage />
  ) : session === undefined ? (
    <p className="muted">Opening Binbin…</p>
  ) : !session ? (
    <LoginPage />
  ) : page === "new" ? (
    <NewSharePage />
  ) : page === "share" && id ? (
    <SharePage id={id} />
  ) : page === "settings" ? (
    <SettingsPage />
  ) : page === "admin" && session.user.admin ? (
    <AdminPage />
  ) : (
    <DashboardPage />
  );
  return (
    <TooltipProvider delay={350}>
      <AppContext.Provider
        value={{
          user:
            session?.user === undefined
              ? session === undefined
                ? undefined
                : null
              : session.user,
          csrf: session?.csrf || "",
          notify,
          signedIn,
          signedOut,
          logout,
          uploads,
          navigate,
        }}
      >
        <a
          className="skip-link"
          href="#app"
          onClick={(event) => {
            event.preventDefault();
            document.getElementById("app")?.focus();
          }}
        >
          Skip to content
        </a>
        <header className="app-header">
          <div className="header-inner">
            <a href="/" className="wordmark" aria-label="Binbin home">
              <svg
                viewBox="0 0 32 32"
                fill="none"
                stroke="currentColor"
                strokeWidth={2}
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M7 5v22h9a7 7 0 0 0 0-14H7m12-8v8m-7 6h5" />
              </svg>
              binbin
            </a>
            <nav id="nav" aria-label="Main navigation">
              {authenticated &&
                (
                  [
                    ["shares", "Shares", Files],
                    ["new", "New share", FilePlus2],
                    ["settings", "Settings", Settings],
                    ...(session.user.admin
                      ? [["admin", "Administration", Shield] as const]
                      : []),
                  ] as const
                ).map(([key, label, Icon]) => (
                  <LinkButton
                    variant={
                      page === key || (page === "share" && key === "shares")
                        ? "secondary"
                        : "ghost"
                    }

                    href={`#${key}`}
                    key={key}
                    aria-current={
                      page === key || (page === "share" && key === "shares")
                        ? "page"
                        : undefined
                    }
                  >
                    <Icon aria-hidden="true" />
                    {label}
                  </LinkButton>
                ))}
            </nav>
            <div id="account-tools">
              <ThemeControl theme={theme} onChange={setTheme} />
              {authenticated && (
                <>
                  <span className="account-name">{session.user.username}</span>
                  <IconButton
                    label="Sign out"
                    data-action="logout"
                    onClick={() => void logout()}
                  >
                    <LogOut aria-hidden="true" />
                  </IconButton>
                </>
              )}
            </div>
          </div>
        </header>
        <Alert
          variant={notice?.error ? "destructive" : "default"}
          id="notice"
          role={notice?.error ? "alert" : "status"}
          aria-live="polite"
          aria-atomic="true"
          className={notice ? `visible${notice.error ? " error" : ""}` : ""}
        >
          {notice && (
            <>
              <AlertDescription className="min-w-0 text-inherit [overflow-wrap:anywhere]">
                {notice.message}
                {notice.href && (
                  <a
                    className="mt-2 block w-fit underline underline-offset-4"
                    href={notice.href}
                  >
                    {notice.href === "#new"
                      ? "Return to files"
                      : "Review draft"}
                  </a>
                )}
              </AlertDescription>
              <AlertAction>
                <IconButton
                  size="icon-xs"
                  label="Dismiss notification"
                  onClick={() => notify("")}
                >
                  <X aria-hidden="true" />
                </IconButton>
              </AlertAction>
            </>
          )}
        </Alert>
        {authenticated && <UploadStatus />}
        <main
          id="app"
          tabIndex={-1}
          key={`${route}:${revision}:${session?.csrf || "guest"}`}
        >
          {content}
        </main>
        <footer>
          <span>Binbin v{__BINBIN_VERSION__}</span>
          <span>Private file sharing</span>
        </footer>
      </AppContext.Provider>
    </TooltipProvider>
  );
}
