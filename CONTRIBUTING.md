# Development

Use Rust 1.88 or newer and Node 22.13 or a current supported Node LTS release. Linux workspace builds require `libdbus-1-dev` and `pkg-config` for the CLI's credential-store backend. A server-only build does not need them.

```sh
npm --prefix web ci
npm --prefix web run build
cd web && npx playwright install chromium && cd ..
cargo run --bin binbin-server -- init-admin --username admin
cargo run --bin binbin-server
```

Read [the architecture](docs/architecture.md) before changing module boundaries. The browser client lives in `web/`; Rust packages live in `crates/`. Run `npm --prefix web run build` and rebuild the server after changing its embedded frontend assets. For hot reload, keep the Rust server on localhost:8080 and run `npm --prefix web run dev`; Vite serves the React UI and proxies API/download requests to that server.

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo +1.88.0 check --locked --workspace --all-targets
cargo machete
npm --prefix web run check
npm --prefix web test
docker compose build
bash scripts/docker-smoke.sh binbin:local
```

`npm --prefix web run format` formats the web package’s sources, styles, and test/tool configuration. `npm --prefix web run lint` checks JavaScript/TypeScript and React hook rules. `npm --prefix web run typecheck` checks the TypeScript contracts. `npm --prefix web test` builds the frontend before running browser scenarios. Rust formatting follows rustfmt; shared workspace lints prohibit unsafe code in application packages.

The browser test server uses temporary storage and port 8086. Its test credentials are confined to the test example and fixtures. Keep that port free when running Playwright. The Docker smoke test creates and removes its own containers and volume; it does not use the application's existing data volume.

GitHub workflow changes can be checked with [actionlint](https://github.com/rhysd/actionlint):

```sh
actionlint .github/workflows/release.yml
```

Publishing happens only when a GitHub Release is published and uses the GitHub repository path automatically. See [the release guide](docs/releases.md) for tags, package visibility, and deployment.

## Changes

Keep business rules in application services and transport policy in the server. Keep filesystem and SQL details behind the core storage boundary. Add regression tests for bugs and meaningful behavior changes; a code move does not need a duplicate test. Preserve the public HTTP response format and persisted records unless a change includes a deliberate migration.

Never add credentials or runtime data to the repository. Use the application's hidden prompts and documented token-input options for local setup and headless clients.
