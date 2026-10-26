import { RefreshError, Panel, Heading, Loading } from "@/components/common";
import { useState } from "react";
import { useApp } from "@/app/context";
import { useRemote } from "@/hooks/use-remote";
import { useTask } from "@/hooks/use-task";
import { api } from "@/lib/api";
import { Field, CheckField } from "@/components/forms";
import { Button } from "@/components/ui/button";
import type { Limits, User } from "@/types/api";
export function AdminPage() {
  const { data: users, error, reload } = useRemote<User[]>("/api/users");
  const limits = useRemote<Limits>("/api/settings");
  const { user, notify, signedOut } = useApp();
  const { run, pending } = useTask();
  const [reset, setReset] = useState<User | null>(null);
  if (!users || !limits.data) return <Loading error={error || limits.error} />;
  const settings = limits.data.settings;
  const fields: [string, string, number | null][] = [
    ["file_limit", "Maximum file size (GiB)", settings.file_limit / 2 ** 30],
    ["share_limit", "Maximum share size (GiB)", settings.share_limit / 2 ** 30],
    [
      "storage_limit",
      "Storage capacity (GiB)",
      settings.storage_limit / 2 ** 30,
    ],
    ["file_count", "Files per share", settings.file_count],
    ["default_days", "Default lifetime (days)", settings.default_days],
    ["max_days", "Maximum lifetime (days)", settings.max_days],
    ["max_downloads", "Maximum download allowance", settings.max_downloads],
  ];
  return (
    <>
      <Heading
        title="Administration"
        description="Manage accounts and storage limits."
      />
      <RefreshError
        error={error || limits.error}
        retry={() => {
          reload();
          limits.reload();
        }}
      />
      <div className="admin-layout">
        <Panel className="work-area" title="Accounts">
          {users.map((account) => (
            <div className="user-row" key={account.id}>
              <span>
                <strong>{account.username}</strong>
                <small>
                  {account.admin ? "Administrator" : "Uploader"} ·{" "}
                  {account.enabled ? "Enabled" : "Disabled"}
                </small>
              </span>
              <div>
                {account.id !== user?.id && (
                  <Button
                    variant="outline"
                    data-action="toggle-user"
                    disabled={pending}
                    onClick={() =>
                      void run(async (signal) => {
                        await api(`/api/users/${account.id}`, {
                          method: "PATCH",
                          body: { enabled: !account.enabled },
                          signal,
                        });
                        reload();
                        setReset(null);
                        notify(
                          "Account updated. Existing sessions and tokens were invalidated.",
                        );
                      })
                    }
                  >
                    {account.enabled ? "Disable" : "Enable"}
                  </Button>
                )}
                <Button
                  variant="outline"
                  data-action="reset-user"
                  disabled={pending}
                  onClick={() => setReset(account)}
                >
                  Reset password
                </Button>
              </div>
            </div>
          ))}
          <div id="password-reset-area">
            {reset && (
              <form
                key={reset.id}
                id="password-reset"
                onSubmit={(event) => {
                  event.preventDefault();
                  const values = new FormData(event.currentTarget);
                  void run(async (signal) => {
                    await api(`/api/users/${reset.id}`, {
                      method: "PATCH",
                      body: {
                        password: values.get("password"),
                        enabled: reset.enabled,
                      },
                      signal,
                    });
                    if (reset.id === user?.id) {
                      signedOut();
                      notify("Password reset. Sign in again.");
                    } else {
                      reload();
                      setReset(null);
                      notify(
                        "Password reset. Existing sessions and tokens were invalidated.",
                      );
                    }
                  });
                }}
              >
                <h3>Set a new password for {reset.username}</h3>
                <Field
                  label="New password"
                  name="password"
                  type="password"
                  minLength={12}
                  maxLength={1024}
                  required
                  autoFocus
                  autoComplete="new-password"
                />
                <Button variant="default" type="submit" disabled={pending}>
                  Reset password and invalidate sessions
                </Button>
              </form>
            )}
          </div>
          <h3>Create account</h3>
          <form
            id="new-user"
            onSubmit={(event) => {
              event.preventDefault();
              const values = new FormData(event.currentTarget);
              const form = event.currentTarget;
              void run(async (signal) => {
                await api("/api/users", {
                  method: "POST",
                  body: {
                    username: values.get("username"),
                    password: values.get("password"),
                    admin: values.has("admin"),
                  },
                  signal,
                });
                form.reset();
                reload();
                notify("Account created.");
              });
            }}
          >
            <Field
              label="Username"
              name="username"
              maxLength={64}
              required
              autoComplete="off"
            />
            <Field
              label="Initial password"
              name="password"
              type="password"
              minLength={12}
              maxLength={1024}
              required
              autoComplete="new-password"
            />
            <CheckField label="Administrator access" name="admin" />
            <Button type="submit" variant="default" disabled={pending}>
              Create account
            </Button>
          </form>
        </Panel>
        <Panel className="share-options" title="Instance limits">
          <form
            id="instance-settings"
            onSubmit={(event) => {
              event.preventDefault();
              const values = new FormData(event.currentTarget);
              const body: Record<string, number | null> = {};
              for (const [key, value] of values)
                body[key] = value
                  ? key.endsWith("limit")
                    ? Math.round(Number(value) * 2 ** 30)
                    : Number(value)
                  : null;
              void run(async (signal) => {
                await api("/api/settings", { method: "PUT", body, signal });
                limits.reload();
                notify("Instance limits saved.");
              });
            }}
          >
            {fields.map(([name, label, value]) => (
              <Field
                key={name}
                label={label}
                name={name}
                type="number"
                min={name.endsWith("limit") ? 1 / 2 ** 30 : 1}
                step={name.endsWith("limit") ? "any" : 1}
                defaultValue={value ?? ""}
                required={[
                  "file_limit",
                  "share_limit",
                  "storage_limit",
                  "file_count",
                ].includes(name)}
                placeholder="Unlimited"
              />
            ))}
            <p className="fine muted">
              Maximums apply when publishing or editing. Changing capacity never
              deletes existing shares.
            </p>
            <Button
              type="submit"
              variant="default"
              className="full"
              disabled={pending}
            >
              Save limits
            </Button>
          </form>
        </Panel>
      </div>
    </>
  );
}
