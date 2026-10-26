# HTTP API

All metadata responses are JSON. Errors have `{ "error": "message" }` and an appropriate HTTP status. Use `Authorization: Bearer TOKEN` for automation. Browser sessions use an HttpOnly, SameSite=Strict cookie; mutations also require `X-Binbin-CSRF` returned by login or `/api/me`. Requests with an Origin must match the configured public origin. There is no cross-origin API access.

## Authentication and administration

- `POST /api/login`: `{ "username": "…", "password": "…" }`; sets cookie, returns user and CSRF value.
- `POST /api/logout`: invalidates current credential and clears the cookie.
- `GET /api/me`: current user and CSRF value.
- `GET /api/tokens`: current account's named tokens; never returns their secrets.
- `POST /api/tokens`: `{ "name": "Laptop" }`; returns a raw `token` once, its hashed ID, and expiry.
- `DELETE /api/tokens/{id}`: revoke your own API token.
- `GET /api/settings`: settings, used reservation bytes, and chunk size.
- `PUT /api/settings`: administrator only. Complete object: `file_limit`, `share_limit`, `storage_limit` (bytes), `file_count`, `default_days`, `max_days`, `max_downloads`. Nullable lifetime/count fields mean unlimited.
- `GET /api/users`: administrator only; safe user records.
- `POST /api/users`: administrator only; `{ "username": "alice", "password": "at least 12 bytes", "admin": false }`.
- `PATCH /api/users/{id}`: administrator only; `{ "enabled": true, "password": "optional replacement" }`. Invalidates all account credentials.

## Private shares

`GET /api/shares` lists accessible shares. `GET /api/shares/{id}` returns one, including upload offsets. Administrator listings include all owners. Views omit password hashes and expose `password_protected` instead.

`POST /api/shares` reserves a private draft:

```json
{
  "title": "Project handover",
  "files": [{ "name": "report.pdf", "size": 12345 }],
  "days": 7,
  "limit": 5,
  "password": null
}
```

Specify `days` and `limit` explicitly; null means unlimited. The browser and CLI read configured defaults first. Filenames must be unique basenames, at most 240 UTF-8 bytes; paths and control characters are rejected.

`PATCH /api/files/{file_id}?offset=0` uploads raw bytes, at most 8 MiB. The result has the committed offset. A mismatched offset returns 409: inspect the draft before retrying. `GET /api/files/{file_id}/proofs` returns SHA-256 digests of committed 8 MiB segments (last segment can be shorter). An empty PATCH with `X-Binbin-Prefix-SHA256` verifies the entire committed prefix for CLI resumption.

`POST /api/shares/{id}/publish` validates contents and prepares any ZIP. It returns the published share and URL. Archive creation can take time for large batches. `PATCH /api/shares/{id}` accepts `title`, nullable Unix-second `expires`, nullable total `limit`, `change_password`, and nullable `password`. `DELETE /api/shares/{id}` permanently revokes the share or removes a draft.

## Public metadata and transfers

`POST /api/public/{id}` with `{ "password": null }` returns public metadata. Supply a password when protected; 401 requests one. Unavailable shares return generic 404 responses.

Create a public transfer with `POST /api/public/{id}/transfers`:

```json
{
  "resource": "FILE_ID_OR_zip",
  "key": "a-client-generated-random-key",
  "password": null
}
```

Keys are scoped to the resource, share version, and current management credential (or public access). Expired keys cannot be reused; create a fresh key for a new transfer. Keys must be 16–200 characters; clients should generate at least 128 random bits. Reuse the key if a request fails. The response contains `token`, `url`, and initial `expires`. Issuance does not consume an allowance. `POST /api/shares/{id}/transfers` is the authenticated management equivalent and bypasses allowances.

`GET /d/{token}` starts or resumes the transfer. This capability URL must be treated as a secret. `HEAD` inspects it without charging. Valid GET bodies consume exactly one allowance per transfer record. `Range: bytes=START-END`, suffix/open ranges, `If-Range`, ETag, `If-None-Match`, and `Content-Length` are supported. Multiple ranges are rejected with 416. Downloads use attachment disposition and no-store caching. A retry uses the same URL without another charge, bounded by its 24-hour lifetime and the share's deadline/version/revocation status.

Login and public password/transfer endpoints have a global 120-request/minute limit per endpoint class. This is intentionally conservative for a private instance. A shared limit permits 16 concurrent response futures across routes; active download bodies continue streaming after their permits are released. No recipient identifiers are retained in rate-limit storage.
