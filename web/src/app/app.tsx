import { LinkButton } from "@/components/link-button";
import { useCallback, useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { AppContext } from "./context";
import {
  api,
  ApiError,
  errorMessage,
  setCsrf,
  setSessionExpiredHandler,
} from "@/lib/api";
import { uploadFiles, type UploadProgress } from "@/transfers/upload";
import { useTheme, type Theme } from "@/hooks/use-theme";
import { Button } from "@/components/ui/button";
import { NativeSelect } from "@/components/ui/native-select";
import { Alert, AlertDescription, AlertAction } from "@/components/ui/alert";
import { LoginPage } from "@/pages/login-page";
import { DashboardPage } from "@/pages/dashboard-page";
import { NewSharePage } from "@/pages/new-share-page";
import { SharePage } from "@/pages/share-page";
import { SettingsPage } from "@/pages/settings-page";
import { AdminPage } from "@/pages/admin-page";
import { PublicPage } from "@/pages/public-page";
import type { Session, Share } from "@/types/api";
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
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<UploadProgress | null>(null);
  const busyRef = useRef(false);
  const acceptedHash = useRef(location.hash);
  const controller = useRef<AbortController | null>(null);
  const generation = useRef(0);
  const { theme, setTheme } = useTheme();
  const notify = useCallback(
    (message: string, error = false) =>
      setNotice(message ? { message, error } : null),
    [],
  );
  const signedOut = useCallback(() => {
    generation.current++;
    controller.current?.abort();
    controller.current = null;
    busyRef.current = false;
    setBusy(false);
    setProgress(null);
    setCsrf("");
    setSession(null);
  }, []);
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
      if (busyRef.current) {
        history.replaceState(
          null,
          "",
          `${location.pathname}${location.search}${acceptedHash.current}`,
        );
        notify("Pause the upload before leaving this page.", true);
        return;
      }
      acceptedHash.current = location.hash;
      setRoute(location.hash.slice(1) || "shares");
      setNotice(null);
    };
    const unloading = (event: BeforeUnloadEvent) => {
      if (busyRef.current) {
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
  }, [notify]);
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
    acceptedHash.current = `#${hash}`;
    history.replaceState(
      null,
      "",
      `${location.pathname}${location.search}#${hash}`,
    );
    setRoute(hash);
    setRevision((n) => n + 1);
  }, []);
  const logout = useCallback(async () => {
    generation.current++;
    controller.current?.abort();
    busyRef.current = false;
    setBusy(false);
    setProgress(null);
    try {
      await api("/api/logout", { method: "POST" });
      signedOut();
    } catch (error) {
      if (!(error instanceof ApiError && error.status === 401))
        notify(errorMessage(error), true);
    }
  }, [notify, signedOut]);
  const pauseUpload = useCallback(() => controller.current?.abort(), []);
  const upload = useCallback(
    async (load: (signal: AbortSignal) => Promise<Share>, files: File[]) => {
      if (busyRef.current) return;
      const abort = new AbortController();
      controller.current = abort;
      busyRef.current = true;
      setBusy(true);
      const owner = generation.current;
      let draft: Share | undefined;
      setProgress({
        label: "Creating private draft…",
        committed: 0,
        total: files.reduce((n, f) => n + f.size, 0),
      });
      try {
        draft = await load(abort.signal);
        abort.signal.throwIfAborted();
        setProgress({
          label: "Checking files…",
          committed: 0,
          total: draft.original_size,
        });
        await uploadFiles(draft, files, abort.signal, setProgress);
        abort.signal.throwIfAborted();
        if (owner === generation.current) {
          busyRef.current = false;
          setBusy(false);
          setProgress(null);
          navigate(`share/${draft.id}`);
          notify("Upload complete. Review your files and publish when ready.");
        }
      } catch (error) {
        if (owner !== generation.current) return;
        busyRef.current = false;
        setBusy(false);
        setProgress(null);
        if (draft) navigate(`share/${draft.id}`);
        notify(
          abort.signal.aborted
            ? "Upload paused. Choose the same files to resume."
            : errorMessage(error),
          !abort.signal.aborted,
        );
      } finally {
        if (controller.current === abort) {
          controller.current = null;
          busyRef.current = false;
          setBusy(false);
          setProgress(null);
        }
      }
    },
    [navigate, notify],
  );
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
    <AppContext.Provider
      value={{
        user:
          session?.user === undefined
            ? session === undefined
              ? undefined
              : null
            : session.user,
        csrf: session?.csrf || "",
        notice,
        notify,
        signedIn,
        signedOut,
        logout,
        busy,
        progress,
        pauseUpload,
        upload,
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
              [
                ["shares", "Shares"],
                ["new", "New share"],
                ["settings", "Settings"],
                ...(session.user.admin ? [["admin", "Administration"]] : []),
              ].map(([key, label]) => (
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
                  {label}
                </LinkButton>
              ))}
          </nav>
          <div id="account-tools">
            <label className="theme-control">
              <span className="sr-only">Color theme</span>
              <NativeSelect
                id="theme"
                aria-label="Color theme"
                value={theme}
                onChange={(e) => setTheme(e.target.value as Theme)}
              >
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </NativeSelect>
            </label>
            {authenticated && (
              <>
                <span className="account-name">{session.user.username}</span>
                <Button
                  variant="ghost"
                  data-action="logout"
                  onClick={() => void logout()}
                >
                  Sign out
                </Button>
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
            </AlertDescription>
            <AlertAction>
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label="Dismiss notification"
                onClick={() => notify("")}
              >
                <X aria-hidden="true" />
              </Button>
            </AlertAction>
          </>
        )}
      </Alert>
      <main
        id="app"
        tabIndex={-1}
        key={`${route}:${revision}:${session?.csrf || "guest"}`}
      >
        {content}
      </main>
      <footer>
        <span>Binbin</span>
        <span>Private file sharing</span>
      </footer>
    </AppContext.Provider>
  );
}
