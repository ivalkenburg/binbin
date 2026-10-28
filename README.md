# Binbin

Private file sharing in Rust. Upload through the browser or CLI, explicitly publish a batch, and send an unguessable link. Shares can require a password and expire by deadline, download allowance, or either condition.

The server stores metadata in SQLite and streams file contents from local disk. Multi-file shares include a prepared, uncompressed ZIP for reliable ranged downloads. One container and one persistent volume run the application; no separate database or Node runtime is needed in production.

See [architecture and repository layout](docs/architecture.md), [development instructions](CONTRIBUTING.md), and the [design system](docs/design.md).

## Run with Docker

Publishing a GitHub Release builds multi-platform images for `ghcr.io/<owner>/<repository>` (tagged with the version and `latest`), attaches CLI binaries, and updates the Homebrew tap. See [GitHub setup, releases, and running a published image](docs/releases.md). For a local build:

```sh
docker compose build
docker compose run --rm binbin init-admin --username admin
docker compose up -d
```

The administrator password is entered through a hidden interactive prompt. Open **http://localhost:8080**. Create additional accounts under Administration. Public registration is disabled.

For deployment, set `BINBIN_PUBLIC_URL=https://files.example.com` in `.env` and place the container behind an HTTPS reverse proxy. The URL must be an origin without a path. Cookies become `Secure` automatically for HTTPS origins. Configure the proxy to stream responses, accept at least 8 MiB request bodies, and allow sufficient publication time for large archives. Do not log `/d/` capability URLs. Compose binds to loopback by default; adapt networking if the reverse proxy runs in another container.

The container runs as UID/GID **10001**. A named volume is configured automatically. Bind mounts must be writable by that user. Run only one server against a data directory.

## Browser workflow

1. Sign in, choose New share, and select or drop files.
2. Set a title, lifetime, download allowance, and optional password (at least 12 characters).
3. Upload, then review and explicitly publish. Add or remove files until publication, preview images, or paste screenshots. Uploads continue while navigating Binbin. After a refresh, select only unfinished originals to resume. Private drafts last 24 hours after creation.
4. Copy the share link. Recipients do not need accounts.

Each public file or ZIP transfer consumes one allowance **when its body starts**, rather than when the link is viewed. Its retry credential permits resumption of that same resource for 24 hours from first transfer, without another charge. A count-exhausted share retains content for outstanding transfers. Time expiry, password changes, and revocation block further requests; an already-streaming response may finish.

Downloads through authenticated management commands bypass allowances. Public-link downloads consume them even for the owner. Expired/revoked shares cannot be restored. File contents are reclaimed by the minute-based cleanup job; minimal history remains for 30 days. See [the lifecycle specification](docs/specification.md).

## CLI

Build with Rust 1.88 or later (the Docker build uses Rust 1.97):

```sh
cargo install --path crates/binbin-cli --bin binbin
binbin --server https://files.example.com login
binbin upload report.pdf notes.txt --title "Project handover" --days 7 --downloads 5
binbin list
binbin inspect SHARE_ID
binbin download SHARE_ID --output handover.zip
binbin fetch https://files.example.com/s/SHARE_ID --output handover.zip
binbin revoke SHARE_ID
```

Create a named API token in browser Settings and paste it into `login`'s hidden prompt. The token is saved in the OS credential store. On Linux, building the default CLI requires D-Bus development files (`libdbus-1-dev` and `pkg-config` on Debian/Ubuntu), and persistent credentials require an unlocked Secret Service. macOS uses Keychain; Windows uses Credential Manager.

For headless use, provide `--token-file /run/secrets/binbin`, `--token-stdin`, or `--prompt-token`. These modes never save another token copy. No silent plaintext fallback exists. Only the server URL is saved in the configuration file. Logging out deletes the local credential; revoke the server token separately in Settings. API tokens expire after one year.

Use `--no-expiry` for unlimited lifetime, `--password` to prompt for a share password, and `--json` for machine-readable output. The CLI reports a private draft ID before uploading; repeat the upload with `--resume DRAFT_ID` and the complete current file set to continue. Both clients compare uploaded prefixes before resuming.

Repeat a failed download command with the same output path to resume. A persistent `.binbin-transfer.lock` file prevents simultaneous downloads from writing the same output; leave it in place while a download may be running. A private `.binbin-transfer.json` sidecar preserves its credential and idempotency key; remove it only when abandoning the transfer. Successful downloads verify SHA-256 and remove the sidecar. If verification fails, remove only the corrupt output and keep its sidecar to retry with the same credential. Existing output files without a matching sidecar are never overwritten. `fetch --password` prompts for a protected share's password.

## Defaults and limits

- 5 GiB per file; 20 GiB of originals and 100 files per share.
- 100 GiB instance capacity, including drafts and prepared ZIPs.
- Seven-day default lifetime; no maximum lifetime or download allowance initially.
- A multi-file share reserves originals plus ZIP overhead: roughly twice its file sizes.

Administrators can change limits. Reservations prevent concurrent uploads from overcommitting configured capacity. Reducing capacity never evicts existing files. Files remain server-readable; use HTTPS and protect the host and backups.

## Local development and verification

```sh
npm --prefix web ci
npm --prefix web run build
cargo run --bin binbin-server -- init-admin --username admin
cargo run --bin binbin-server
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
npm --prefix web run check
cd web && npx playwright install chromium && cd ..
npm --prefix web test
docker build -t binbin:local .
bash scripts/docker-smoke.sh binbin:local
```

The React browser client uses shadcn/ui Base UI components themed with the supplied light/dark theme. Run `npm --prefix web run dev` for hot reload against a local server on port 8080. Production assets are built by Vite and embedded in the Rust binary.

The browser tests start an isolated fixture on port 8086, with temporary data and test-only credentials. Rust tests cover authentication, CSRF, permissions, publication, checksums, ZIP integrity, deadlines, concurrent allowances/reservations, cleanup, HTTP ranges, token lifecycle, and real CLI workflows. Browser tests check every page and transient state at desktop, tablet, mobile, and 320px widths in both light and dark modes, including download feedback, theme persistence, long filenames, and document/container overflow. Focused browser regressions cover delayed navigation, upload cancellation, repeated download clicks, denied storage, password changes, and keyboard focus. The release workflow runs the Rust tests and a container smoke test before publishing.

Configure `BINBIN_DATA` (default `data`), `BINBIN_BIND` (default `0.0.0.0:8080`), `BINBIN_PUBLIC_URL` (default `http://localhost:8080`), and `RUST_LOG`. `/healthz` is the health endpoint. [HTTP API documentation](docs/api.md) describes the client protocol.

The server writes timestamped event logs to stdout. Events include sign-in successes and failures, sign-out, account and API token changes, share creation/publication/updates/deletion, completed file uploads, download transfer preparation and starts, and instance settings changes. Logs include operation IDs and transfer sizes where applicable; passwords, session cookies, API token secrets, and download capability URLs are excluded. Set `RUST_LOG` to control verbosity (default `binbin=info`).

## Back up and restore

The reliable full backup procedure is to **stop the server**, copy the entire persistent data directory or Docker volume (including SQLite, WAL files if present, and share directories), then restart. Use your volume backup tool while the service is stopped. Store backups outside the live volume. A database-only backup cannot restore uploaded contents.

For a local checkout, store the archive outside the source directory:

```sh
# Stop the running Binbin process before copying.
tar -czf ../binbin-backup.tgz data
# Restart Binbin after the copy completes.
```

Restore into an empty data directory while the server is stopped, preserve file ownership (10001:10001 in Docker), and start the same application version. Backups contain files and authentication records; keep them private. `binbin-server backup-db DESTINATION` creates a consistent database-only snapshot for inspection. It does not replace the full backup procedure.

The initial release intentionally excludes public registration, uncharged recipient previews, S3, OIDC, malware scanning, per-user quotas, notifications, and a recovery bin.
