# Architecture

Binbin is a Cargo workspace with a static browser client. Runtime data stays outside the source tree in the configured data directory.

## Repository layout

```text
crates/
  binbin-core/       Application rules, models, crypto, SQLite, and file storage
  binbin-server/     Axum routes, HTTP policy, embedded assets, and server lifecycle
  binbin-cli/        Arguments, API client, credentials, and file transfers
web/
  index.html        Browser entry point
  src/              React and TypeScript
    app/            Application provider, session lifecycle, routing and shell
    pages/          Route components
    components/     Shared forms, file picker, transfer controls and feedback
      ui/           shadcn/ui primitives
    hooks/          Abortable data/actions and theme preferences
    transfers/      Upload verification and download capability workflows
    lib/            Typed API calls, formatting and class merging
    types/          API response contracts
  dist/             Generated Vite bundle, ignored by Git
  styles/           Tokens, base styles, layout, components, and responsive rules
  tests/e2e/        Browser scenarios and shared test support
  ci/setup/         Composite GitHub action to build web assets
docs/               Product behavior, architecture, API, design, and operations
scripts/            Repository and container verification utilities
.github/workflows/  Continuous integration
```

Cargo manifests, the Dockerfile, and Compose configuration stay at the repository root. The web package owns its npm manifests, Vite, TypeScript, shadcn, ESLint, Prettier, and Playwright configuration in `web/`. Its composite GitHub setup action lives in `web/ci/setup/`; the release workflow entry point remains in `.github/workflows/`.

## Rust boundaries

`binbin-core` owns account policy, share state, storage reservations, expiration, transfer allowances, and cleanup. Services use the private storage module; HTTP handlers never run SQL. SQLite records and file layouts remain compatible with the original single-crate application.

`binbin-server` translates HTTP requests to application calls and errors to HTTP responses. Its handlers are separated by resource. Authentication, request policy, byte ranges, and embedded assets have their own modules. Blocking application work runs on Tokio's blocking pool. The executable only starts the server runtime.

`binbin-cli` owns its command parser, HTTP client, OS credential storage, and transfer state. It has a thin executable entry point and separate upload/download implementations. Keeping it in a separate package prevents the server build from depending on desktop credential-store libraries.

## Browser client

The frontend uses React, TypeScript, Vite, Tailwind CSS, and shadcn/ui Base UI components. React owns rendering and interaction state across every route. Route-scoped reads and form mutations are canceled on unmount. An application-owned `UploadManager` retains local File handles, serializes draft mutations, reconciles uncertain edit results and fresh draft views using membership versions, and runs sequential per-draft file queues across navigation. Queue requests end at session termination; refresh/close warns during active work. After reload, completed remote files remain available and only unfinished files require reselection. Download capability issuance remains coalesced by resource, with persisted retry keys and a memory fallback.

`web/components.json` configures the shadcn registry and aliases. Primitives live in `web/src/components/ui/`, while application components handle labels, file selection, progress, and retry feedback. Semantic shadcn variables define the supplied light/dark theme in `web/styles/tokens.css` and `main.css`. System/light/dark preference changes update the same tokens and the Tailwind dark variant.

Run `npm --prefix web ci && npm --prefix web run build` before compiling the Rust server. Vite emits hashed assets and a manifest into `web/dist/`; the server build watches and embeds that directory. HTTP asset tests read the manifest instead of assuming filenames. Docker builds the frontend in a Node stage, then embeds its output during the Rust build. Production still runs one Rust binary, with no Node service or external asset dependencies.

The content security policy keeps scripts and stylesheets on the same origin. Blob image URLs support local previews and are revoked when their components unmount. Authenticated remote previews identify raster image signatures and stream from a file handle opened during the authorization transaction. Inline style attributes are allowed for Base UI control internals and bounded progress sizing; inline scripts and style elements remain blocked.

## Data and concurrency

SQLite transactions serialize storage reservations, draft collection edits, and the first charge for a download. Startup applies appended, numbered SQL migrations transactionally using SQLite user_version; released migration files stay unchanged, and newer database versions are rejected. Removed draft files use temporary `removed_file` objects in the existing object store, with their per-share index added by migration 0002. Undo restores the same file ID and committed offset; its bytes stay reserved until purged. Publication first commits the end of Undo, then unlinks those bytes in a subsequent transaction so a failed preparing transaction cannot lose restorable files. Cleanup also removes orphan files from interrupted draft additions. Upload offsets advance only after durable writes. File sets become immutable before archive preparation; expensive archive work happens outside the database transaction. Cleanup respects the lifetime of eligible transfer credentials, including the current validity of a management credential. Resume verification captures a file handle and committed offset inside a short transaction, then hashes the prefix outside the database lock. Credential issuance rechecks its authentication source before committing. Public password verification also runs outside the database mutex and rechecks the captured password/version before returning metadata or issuing a transfer. Cleanup uses a set of eligible share/version pairs for retention and keeps expired-key tombstones until their share version becomes unusable.

On Unix, file creation and archive publication flush the relevant directory entries before metadata becomes committed. Other platforms flush file contents but have no portable directory-flush guarantee in the standard library. Retry sidecars use private sibling temporary files and atomic replacement so failed updates preserve the previous credential. Per-output OS locks prevent competing CLI processes from replacing each other’s transfer state or output. Browser transfers coalesce pending issuance and validate saved capabilities without charging an allowance.

The source reorganization does not move the Docker data volume or change the public API. Full backup and restore procedures are described in the README and lifecycle details in [specification.md](specification.md).

## Tests

Core unit tests are grouped by service with shared private fixtures. Server tests cover HTTP behavior and asset delivery. CLI integration tests run the actual executable against an ephemeral server. Playwright scenarios live in `web/tests/e2e/` and use the server package's isolated test example, not the running Docker application.
