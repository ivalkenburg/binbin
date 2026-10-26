import { Panel } from "@/components/common";
import { useState } from "react";
import { File } from "lucide-react";
import { useApp } from "@/app/context";
import { useTask } from "@/hooks/use-task";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Field } from "@/components/forms";
import type { Session } from "@/types/api";
export function LoginPage() {
  const { signedIn } = useApp();
  const { run, pending } = useTask();
  const [error, setError] = useState("");
  return (
    <Panel className="login">
      <div className="login-mark">
        <File aria-hidden="true" />
      </div>
      <h1>Sign in to Binbin</h1>
      <p className="muted">Your files. Shared on your terms.</p>
      <form
        id="login"
        onSubmit={(event) => {
          event.preventDefault();
          const data = new FormData(event.currentTarget);
          setError("");
          void run(
            async (signal) =>
              signedIn(
                await api<Session>("/api/login", {
                  method: "POST",
                  body: {
                    username: data.get("username"),
                    password: data.get("password"),
                  },
                  signal,
                }),
              ),
            setError,
          );
        }}
      >
        <Field
          label="Username"
          name="username"
          autoComplete="username"
          maxLength={64}
          required
          autoFocus
        />
        <Field
          label="Password"
          name="password"
          type="password"
          autoComplete="current-password"
          maxLength={1024}
          required
        />
        <Button
          className="full"
          variant="default"
          type="submit"
          disabled={pending}
        >
          {pending ? "Signing in…" : "Sign in"}
        </Button>
        <p className="form-error" id="login-error" role="alert">
          {error}
        </p>
      </form>
      <p className="muted fine login-note">
        Need an account? Contact your administrator.
      </p>
    </Panel>
  );
}
