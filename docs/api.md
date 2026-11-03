# HTTP API

All metadata responses are JSON. Errors have `{ "error": "message" }` and an appropriate HTTP status. Browser sessions use an HttpOnly, SameSite=Strict cookie; mutations also require `X-Binbin-CSRF` returned by login or `/api/me`. Requests with an Origin must match the configured public origin. There is no cross-origin API access.

## Authentication and administration

- `POST /api/login`: `{ "username": "…", "password": "…" }`; sets cookie, returns user and CSRF value.
- `POST /api/logout`: invalidates current credential and clears the cookie.
- `GET /api/me`: current user and CSRF value.
- `GET /api/settings`: settings, used reservation bytes, and chunk size.
- `PUT /api/settings`: administrator only. Complete object: `file_limit`, `share_limit`, `storage_limit` (bytes), `file_count`, `default_days`, `max_days`, `max_downloads`. Nullable lifetime/count fields mean unlimited.
- `GET /api/users`: administrator only; safe user records.
- `POST /api/users`: administrator only; `{ "username": "alice", "password": "at least 12 bytes", "admin": false }`.
- `PATCH /api/users/{id}`: administrator only; `{ "enabled": true, "password": "optional replacement" }`. Invalidates all account credentials.

## Private shares

`GET /api/shares` lists accessible shares. `GET /api/shares/{id}` returns one, including upload offsets. Administrator listings include all owners. Views omit password hashes and expose `password_protected` instead. Private views include `server_time` (Unix seconds) for relative Undo timing; successful collection edits increment `version`, so clients can discard stale membership snapshots.

`POST /api/shares` reserves a private draft. Optional `delete_after_download` defaults to false and must be paired with `limit: null` when enabled:

```json
{
  "title": "Project handover",
  "files": [{ "name": "report.pdf", "size": 12345 }],
  "days": 7,
  "limit": 5,
  "password": null
}
```

Specify `days` and `limit` explicitly; null means unlimited. The browser reads configured defaults first. Filenames must be unique basenames, at most 240 UTF-8 bytes; paths and control characters are rejected.

`PATCH /api/files/{file_id}?offset=0` uploads raw bytes, at most 8 MiB. The result has the committed offset. A mismatched offset returns 409: inspect the draft before retrying. `GET /api/files/{file_id}/proofs` returns SHA-256 digests of committed 8 MiB segments (last segment can be shorter). An empty PATCH with `X-Binbin-Prefix-SHA256` verifies the entire committed prefix for resumption.

`PATCH /api/shares/{id}/files` atomically edits an unexpired private draft. Omit unused arrays:

```json
{
  "add": [{ "name": "another.png", "size": 1024 }],
  "remove": ["FILE_ID"],
  "restore": []
}
```

The response is the updated share. Existing file IDs and committed offsets stay intact. Removed files leave `files` immediately and appear in `removed_files` as `{ "file": { ... }, "removed_at": UNIX_SECONDS }` for eight seconds. Restore within that window to keep their uploaded bytes. Their bytes and names remain reserved during Undo; cleanup releases them afterward. Publication commits all pending removals. Drafts may temporarily contain no files, but publication requires at least one complete file. File edits enforce ownership, filename uniqueness, count, per-file/share sizes, and storage capacity under the same transaction as reservations. Lowered administrator caps still allow edits that reduce or preserve the existing count, sizes, and reservation; newly added or restored files must meet the current per-file cap. Published and preparing shares reject collection edits.

`GET /api/files/{file_id}/preview` streams a complete image to its owner or an administrator, without a transfer credential or download charge. PNG, JPEG, GIF, WebP, and AVIF are identified from file signatures; other contents return 415. Incomplete, removed, expired, and inaccessible files cannot be previewed. Responses use the detected image MIME type, inline disposition, no-store caching, and nosniff. Public recipients have no preview endpoint.

`POST /api/shares/{id}/publish` validates contents and prepares any ZIP. It returns the published share and URL. Archive creation can take time for large batches. `PATCH /api/shares/{id}` accepts `title`, nullable Unix-second `expires`, nullable total `limit`, `change_password`, and nullable `password`. For an unexpired draft it also accepts nullable `days` and `delete_after_download`; supply `days` when editing draft settings. The deletion policy cannot change after publication. Enabled deletion policies reject a non-null allowance. `DELETE /api/shares/{id}` permanently revokes the share or removes a draft.

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

`GET /d/{token}?inline=true` uses inline disposition so the browser can open supported images, PDF, text, audio, and video. Active document formats such as HTML and SVG are served as plain text under a sandbox policy; unknown formats use `application/octet-stream` and may download instead. This uses the same transfer credential and allowance as the attachment URL, including for ranges and retries. The filename link opens this URL in a new tab; the Download button keeps the attachment behavior.

Login and public password/transfer endpoints have a global 120-request/minute limit per endpoint class. This is intentionally conservative for a private instance. A shared limit permits 16 concurrent response futures across routes; active download bodies continue streaming after their permits are released. No recipient identifiers are retained in rate-limit storage.

### Delete after download

Private and authenticated public views expose `delete_after_download` and `downloading`. The latter includes an interrupted transfer's unexpired reservation. Public password gates expose neither field before successful authentication. Ordinary shares retain their existing transfer behavior described above.

For a deletion share, public transfer issuance accepts only its sole file ID or `zip` for multiple files. Other resources and `?inline=true` return 400. Management transfers remain exempt. A valid public GET obtains one exclusive streaming lease; simultaneous requests, including reuse of the same URL, return 409. The server retains delivered ranges for resumptions under the same credential. After an abrupt restart, a requested range beginning beyond the durable contiguous prefix receives a full 200 response without Content-Range, making native clients restart instead of silently completing with a coverage gap. Deliberate disjoint ranges remain supported after clean response completion or interruption. Interrupted transfer credentials last 15 minutes after the last checkpoint/disconnection; expired credentials cannot be reused. HEAD/304/416 never reserve or extend a reservation. Deletion GET responses use streaming framing without `Content-Length` so the body must reach EOF before completion; HEAD retains the resource length.

Completing the entire resource immediately revokes access and purges all live share records and files without retained history. Expiration and manual deletion also purge these shares. Cleanup retries failures without restoring access; subsequent requests return generic 404.

`GET /d/{token}/status` subscribes to a public deletion transfer's live status. `GET /api/shares/{id}/status` is the owner/administrator equivalent. Both stream newline-delimited JSON (`application/x-ndjson`), beginning with `{"state":"ready"}`. Later states are `available`, `downloading`, `interrupted`, `deleting` (access revoked while cleanup retries), and terminal `blocked`, `deleted`, or `unavailable`. Subscribe before requesting the download to observe even immediate completion. The subscription itself does not reserve or charge a transfer. Close it when the page leaves; no receipt or observer is persisted after the connection ends. Proxies must stream these responses without buffering.
