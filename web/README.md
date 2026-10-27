# Web client

React, TypeScript, Vite, Tailwind CSS, and shadcn/ui Base UI. This directory owns the browser package, build configuration, formatting/linting, browser tests, and the composite CI setup action.

From this directory:

```sh
npm ci
npm run dev
npm run check
npm test
```

Development proxies API and download requests to the Rust server at `http://localhost:8080`. The footer version is read from `workspace.package.version` in the root Cargo.toml at build time. Build the assets with `npm run build` before compiling the server; it embeds `dist/`. Run Rust and Docker commands from the repository root. Root-level npm commands use `npm --prefix web …`.

Browser tests build the assets and start the isolated Rust fixture on port 8086 with temporary storage. Install Chromium once with `npx playwright install chromium`.

`components.json` selects the official `base-nova` registry. Components live in `src/components/ui/`; application composition lives one level above. Theme values are in `styles/tokens.css`, with semantic Tailwind mappings in `styles/main.css`. Application CSS handles task layouts and responsive behavior without restyling the component primitives.

See [development instructions](../CONTRIBUTING.md), [architecture](../docs/architecture.md), and [design system](../docs/design.md).
