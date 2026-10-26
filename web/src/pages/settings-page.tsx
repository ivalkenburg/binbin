import { RefreshError, Panel, Heading, Loading } from "@/components/common";
import { useRef, useState } from "react";
import { useRemote } from "@/hooks/use-remote";
import { useTask } from "@/hooks/use-task";
import { useApp } from "@/app/context";
import { api } from "@/lib/api";
import { date } from "@/lib/format";
import { Field } from "@/components/forms";
import { Button } from "@/components/ui/button";
import type { Token } from "@/types/api";
export function SettingsPage() {
  const { data: tokens, error, reload } = useRemote<Token[]>("/api/tokens");
  const { run, pending } = useTask();
  const { notify } = useApp();
  const [secret, setSecret] = useState("");
  const input = useRef<HTMLInputElement>(null);
  if (!tokens) return <Loading error={error} />;
  return (
    <>
      <Heading
        title="Settings"
        description="Manage API access for your devices and scripts."
      />
      <RefreshError error={error} retry={reload} />
      <Panel className="work-area narrow" title="API tokens">
        <p className="muted">
          Create a separate token for each device or script. Tokens inherit your
          permissions and expire after one year.
        </p>
        <form
          id="new-token"
          className="inline-form"
          onSubmit={(event) => {
            event.preventDefault();
            const values = new FormData(event.currentTarget);
            void run(async (signal) => {
              const created = await api<{ token: string }>("/api/tokens", {
                method: "POST",
                body: { name: values.get("name") },
                signal,
              });
              setSecret(created.token);
              reload();
            });
          }}
        >
          <Field
            label="Token name"
            name="name"
            placeholder="e.g. Work laptop"
            maxLength={100}
            required
          />
          <Button variant="default" type="submit" disabled={pending}>
            Create token
          </Button>
        </form>
        <div id="token-result" role="status">
          {secret && (
            <div className="token-reveal">
              <strong>Copy this token now. It won’t be shown again.</strong>
              <Field
                label="API token"
                id="token-value"
                ref={input}
                readOnly
                value={secret}
              />
              <Button
                variant="outline"
                data-action="copy-token"
                onClick={() =>
                  void run(async () => {
                    input.current?.select();
                    if (navigator.clipboard)
                      await navigator.clipboard.writeText(secret);
                    else document.execCommand("copy");
                    notify("Copied to clipboard.");
                  })
                }
              >
                Copy token
              </Button>
              <Button
                variant="outline"
                data-action="hide-token"
                onClick={() => setSecret("")}
              >
                I’ve saved it
              </Button>
            </div>
          )}
        </div>
        {tokens.map((token) => (
          <div className="token-row" key={token.id}>
            <span>
              <strong>{token.name}</strong>
              <small>Expires {date(token.expires)}</small>
            </span>
            <Button
              variant="destructive"
              data-action="revoke-token"
              disabled={pending}
              onClick={() =>
                void run(async (signal) => {
                  await api(`/api/tokens/${token.id}`, {
                    method: "DELETE",
                    signal,
                  });
                  reload();
                  notify("Token revoked.");
                })
              }
            >
              Revoke
            </Button>
          </div>
        ))}
        {!tokens.length && <p className="muted">No API tokens yet.</p>}
        <h3>Use the CLI</h3>
        <p>
          Run <code>binbin --server {location.origin} login</code>, then paste
          your token into the hidden prompt.
        </p>
        <p className="fine muted">
          The CLI uses your operating system’s credential store. For servers and
          CI, use <code>--token-file</code> or <code>--token-stdin</code>.
        </p>
      </Panel>
    </>
  );
}
