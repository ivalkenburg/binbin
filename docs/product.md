# Binbin

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Rust server and CLI, SQLite, persistent local files, one Docker container. The web client uses React, TypeScript, Vite, Tailwind CSS, and shadcn/ui Base UI (base-nova) with @base-ui/react. Browser package manifests, tooling, and tests live in web/.

## Users

Private, administrator-created uploader accounts. Recipients download without accounts through unguessable links and optional share passwords. Administrators manage every account and share.

## Product Purpose

Upload a fixed batch, publish a link, and control its lifetime by deadline and download allowance. Browser and CLI support interrupted transfers.

## Operating Context

Self-hosted behind an HTTPS reverse proxy. Linux, macOS, and Windows CLI. Single application instance; no public registration.

## Capabilities and Constraints

Private editable resumable drafts; uploader image previews; additive file selection and screenshot paste; explicit publication; prebuilt uncompressed ZIPs; storage reservations; configurable limits; dashboard; named API tokens; 30-day minimal history. Recipients can open published files in the browser using the same transfer allowance as downloads. See ../README.md and specification.md for the agreed lifecycle and test coverage. No separate uncharged recipient previews, S3, scanning, OIDC, recovery bin, notifications, or per-user quotas in this release.

## Brand Commitments

Name: Binbin. Compact responsive task interface, searchable shares, system light/dark theme, keyboard navigation, clear progress and cancellation. These choices were explicitly approved in the grilling interview. The latest approved direction uses shadcn/ui Base UI components with the user-supplied OKLCH light/dark theme: neutral surfaces, green primary actions, semantic destructive treatments, compact feedback, and no unnecessary horizontal scrolling. This replaces the earlier strictly neutral palette.

## Product Principles

- A link reveals only what the uploader published.
- Expiration and reservations hold under concurrent requests.
- Retrying a transfer preserves its identity and charge.
- Progress and failures explain the next available action.

## Accessibility & Inclusion

Semantic controls, visible focus, keyboard access, reduced-motion support, responsive layouts.
