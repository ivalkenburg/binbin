# Agreed lifecycle

## Roles and credentials

Private administrator-created accounts. Users manage their own shares; administrators manage and download all shares and configure limits. Account disabling and password resets invalidate all account sessions and API tokens without revoking published shares. An administrator cannot disable their own account. Browser sessions last seven days. Named API tokens last one year and are individually revocable. Passwords use Argon2id; high-entropy session/API/transfer credentials are stored as SHA-256 hashes. Cookie mutations require CSRF verification and requests with a foreign Origin are rejected.

## Drafts, publication, and storage

Drafts are private and uploadable for 24 hours after creation. Clients declare an initial manifest; each safe, unique basename gets an internal file ID. Until publication, owners and administrators can atomically add and remove files while retaining other files’ IDs and upload progress. Removal offers an eight-second Undo window; retained bytes and names remain reserved until that window ends or publication commits the removal. Empty drafts cannot be published. Files are uploaded in chunks of at most 8 MiB with exact offsets. Committed offsets follow durable writes; an interrupted uncommitted tail is truncated on retry. Resume compares committed contents with local prefixes.

The browser appends selections, drops, and pasted screenshots; duplicate names receive numbered suffixes. Compact rows show image thumbnails or file-type icons, size, transfer state, and individual actions. Image previews also work for completed uploads when returning to a private draft. Preview requests require owner/administrator credentials and serve only recognized raster image signatures, without using a download allowance. The initial upload is explicit. New files join a running queue; additions after it finishes require another upload action. A failed file does not stop the others. Publication requires every remaining file to be uploaded successfully. Navigation within Binbin preserves local files and continues transfers, with a persistent indicator; closing or refreshing warns during active work. After reload, only unfinished files need reselection, and their committed prefixes are verified.

Reservations include originals plus a conservative ZIP allowance for multi-file shares and originals held for Undo. Drafts participate in capacity checks. Publication freezes the file set before verifying all lengths, computing SHA-256 digests, and preparing an uncompressed ZIP before making files public. ZIP64 handles large files. The expiration clock starts on publication. Publication rechecks administrator lifetime/download caps. The fixed file set cannot be changed afterward. Active shares can change title, password, deadline, and total allowance within administrator maxima. Lowering an allowance to its consumed count expires the share.

## Public downloads

Opening a share page consumes no allowance. Password-protected pages reveal no metadata before verification. A random share ID grants access when no password is required. Filenames are never used as storage paths.

A client creates a resource-specific transfer using a random idempotency key. The same key, resource, share version, and access scope return the same transfer. Management access scope includes its independently revocable credential. Expired keys retain tombstones while their share version can issue transfers, so expired URLs cannot be revived. New public capabilities use a separate namespace from legacy records; existing valid legacy capabilities retain their retry behavior. Issuance is free; the first valid GET body request atomically charges one allowance, even if the connection subsequently fails. HEAD, invalid ranges, and conditional responses do not charge. Individual files and ZIPs each consume one allowance. Authenticated management transfers bypass counts.

A charged transfer can retry for 24 hours from first body request. Strong ETags and HTTP byte ranges preserve file and ZIP offsets. If-Range mismatches return a full response, and clients restart rather than append. A transfer covers exactly one file or ZIP. Completion means server delivery or client digest verification, never proof that a person retained a file.

## Expiration and deletion

### Delete after download

This optional policy defaults to false and is independent of an ordinary one-download allowance. It replaces the numeric allowance. Owners and administrators may change draft settings, including this policy, before publication; publication locks the policy. Existing shares keep their original behavior. Administrator lifetime limits still apply, and the policy counts as one download when validating administrator download maxima.

The public page and password check never start a transfer. Password gates hide the policy and all file metadata until unlocked. A recipient explicitly downloads a single-file share as an attachment or a multi-file share as its complete ZIP. Individual multi-file requests and inline previews are rejected by the server. Authenticated management previews/downloads do not trigger deletion; using the public link follows recipient rules for everyone.

The first valid GET reserves the share for that transfer credential. Only one public response may be active, even with the same credential; competing requests return 409. HEAD, conditional 304, and invalid ranges do not reserve, consume, or extend the reservation. Delivered byte ranges are merged, including resumed responses; duplicate or overlapping ranges cannot pretend to cover missing bytes. The server deletes only when a response reaches EOF and the same reservation has delivered the entire resource, including zero-byte files. Interrupted responses retain their delivered ranges and reserve the share for the same browser's saved transfer for 15 minutes. After that window the reservation releases, its credential expires, and a new transfer starts with fresh progress. Active responses remain exclusive even if they stall beyond the retry window. Checkpoints survive process restarts. If an abrupt restart loses progress since the last checkpoint, a resume beyond the durable contiguous prefix receives a full 200 response so the browser restarts; completing that delivery still deletes the share. Deliberate disjoint ranges from completed or cleanly interrupted responses retain their range semantics.

Completion first commits irrevocable access removal, then removes original files, ZIPs, file records, removed draft files, the share record and password, all transfer credentials, and the reservation. Completed delivery purges only that share; partial-range EOF does not run instance maintenance. Cleanup retries failed physical deletion independently per share while access remains revoked, so an unrelated filesystem failure cannot roll back or block these removals. Expiration, abandonment, and manual deletion also purge these shares without retaining the ordinary 30-day owner/admin history. Expiry cleanup runs on the regular maintenance schedule. Already-open response handles may finish after manual revocation or expiry.

Only live pages subscribed before deletion receive the transient “Deleted after download” receipt; it is never stored as history. Fresh visits receive generic unavailability with no title, filenames, or deletion reason. Owner/admin lists remove the share. Delivery means the server finished producing the response body, not proof of client storage or reading; proxy/network buffering can outlive server delivery. File unlinking and record deletion are not secure disk erasure. Infrastructure backups/logs and recipient copies retain their own lifetimes.

### Ordinary shares

Any configured deadline or count threshold ends the share. Defaults: seven days, unlimited downloads. Null limits mean unlimited. Both conditions can be absent. Positive lifetime/download maximums can be imposed by administrators at publication/edit time.

Count exhaustion blocks new transfers while existing charged credentials retain access and keep contents on disk for their remaining lifetime. Time expiry blocks new requests and retries. Password changes invalidate existing credentials; their consumed allowances are not refunded. Manual revocation blocks retries immediately. Already-open responses may finish in all cases.

Cleanup runs every minute and at startup. It removes abandoned drafts and expired/revoked content once no eligible transfer requires it. There is no restoration flow. Metadata history retains only title, owner, sizes, dates, count, and deletion reason for 30 days; file records are removed with contents. Recipient IP addresses are not stored in application history. Reverse-proxy logs and infrastructure backups have their own retention policies.

## Deployment

One Rust server process, SQLite, and local persistent storage. Docker runs unprivileged behind a separately managed HTTPS proxy. Defaults are 5 GiB/file, 20 GiB/share, 100 files/share, and 100 GiB total reserved capacity, all configurable. Transfers stream from disk and upload requests are bounded to 8 MiB. File contents are readable by the server operator.
