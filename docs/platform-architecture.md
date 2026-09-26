# Platform Architecture

Current ownership and runtime contracts for Windows 10+ and macOS 13+. Code and
manifests are authoritative. See [extension lifecycle](extension-lifecycle.md),
[design system](design-system.md) and [TODO](tasks.md).

Prioritize runtime performance, then UI/UX, then memory efficiency. Before 1.0,
replace superseded designs directly. Keep internal versions at their initial values;
no compatibility paths, migrations or automatic data resets.

## Responsibility boundaries

| Location              | Responsibility                                                                                 |
| --------------------- | ---------------------------------------------------------------------------------------------- |
| engine/foundation     | Product identity, extension IDs and diagnostics                                                |
| engine/platform       | Native mechanisms: containment, file replacement, launch/reveal and OS integration             |
| Other engine crates   | Search, protocol, supervision, configuration, storage and package policy, independent of Tauri |
| apps/extensions       | Domain capabilities and their discovery/capture adapters                                       |
| apps/desktop/shell    | Tauri commands, channels, windows, permissions, Isolation and WebView sessions                 |
| apps/desktop/frontend | One Svelte renderer for host and declarative extension surfaces                                |
| tooling               | Development, build and validation; never an installed-app dependency                           |

Extensions provide all domain capabilities. Built-in provenance grants no runtime
shortcut. Native adapters own mechanisms, while callers own transaction, lifecycle,
cancellation and failure policy. Reject unsupported platforms explicitly.

## IPC and execution authority

Frontend Tauri access uses the typed bridge and explicit shell permissions. Rust
validates requests against the WebView session, route, current action metadata and
extension permissions. Extensions provide bounded declarative data, never frontend
code or DOM/WebView access.

Invoke replies acknowledge submission. Authoritative search/navigation and Settings
state arrive through separate session Channels; queued delivery is not receipt.
Root execution binds to the immutable delivered search snapshot. Its replacement
invalidates reviewed targets even within one query. Menus and confirmation bind to
exact revisions. Ordinary queued view input retains stable IDs and is revalidated
under the shared operation/invalidation lock. Never retarget stale actions.

The view-input FIFO coalesces only adjacent unsent selections for the same route.
Actions are barriers. Overflow, replaced routes and unavailable targets fail
explicitly. Blocking input waits for RPC completion and the correlated Channel
revision in either order; selection remains nonblocking. Preserve accepted work,
concrete failures and backpressure.

Nanika frames use a 32-bit byte length with no additional application byte quota.
Receivers grow buffers with received bytes, not the claimed header size. Encoding,
allocation, incomplete frames and I/O failures remain errors. Configuration and
catalogs have no aggregate byte quota; script discovery has no entry-count ceiling.
Schema validation, image bounds and bounded queues remain independent constraints.

## Settings operations

Settings submits one property key and value. Rust validates schema and visibility,
merges it into authoritative configuration and reserves one configuration/lifecycle
operation per extension. Other extensions remain independent. The frontend admits
one edit per field and serializes distinct fields in that extension's domain.

Commit preparation validates and converts the complete field before no-op detection;
the writer receives that same value. No-op detection compares saved and presentation
values and preserves unresolved application failures. Failed validation retains the
draft without retrying on navigation. Schema-valid keys use reactive maps. Hidden
platform fields are preserved when omitted and validated when supplied.

Each property declares `persistence`:

| Mode          | Contract                                                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `beforeApply` | Persist intent, then apply. Failure retains saved intent; inactive extensions receive it on activation.                              |
| `afterApply`  | Confirm live application, then persist. Activate on-demand extensions for confirmation; disabled/unavailable extensions cannot save. |

Results separate presentation `values`, `saved`, nullable `effective` and error.
`effective: null` means unconfirmed. If application succeeds but persistence fails,
report effective new values and saved old values without inventing rollback.
`ConfigurationApplied` confirms the entire snapshot after domain work finishes;
queueing, progress and initialization data do not prove application.

`ConfigurationProgress` carries request ID, bounded label and completed/total units;
a null total is indeterminate. Settings delivery permits one unacknowledged progress
message and retains the latest pending progress per extension in arrival order.
Lifecycle has a separate single in-flight snapshot. Both use
`acknowledge_settings_delivery` with a shared sequence never reused across sessions.
Terminal results bypass progress backpressure and discard obsolete progress. Sends
occur outside the state lock; there is no delivery timer or retry. UI feedback is
specified in the [design system](design-system.md#settings).

Hiding Settings or dropping a receipt does not cancel accepted work. Shutdown closes
admission, interrupts extension work and waits for configuration transactions.
Windows startup and macOS login items remain native-service-owned. Reads and writes
share one serial domain; returning to Settings/General refreshes OS state. Failed
writes query actual state, and stale reads cannot overwrite queued edits.

## Persistent storage

Host and extensions own separate databases on background workers. `engine/database`
only configures SQLite and admits the current schema. Owners choose transactions;
search and native event loops perform no database I/O. Runtime DBs use STRICT tables,
WAL, foreign keys, NORMAL synchronization and a bounded busy wait. NORMAL preserves
consistency but may lose recent commits after OS/power failure.

| Database                    | Durable data                                                   |
| --------------------------- | -------------------------------------------------------------- |
| `nanika.db`                 | Extension inventory/provenance, input history and action usage |
| `com.nanika.application.db` | Source records keyed by `(root_key, entry_id)`                 |
| `com.nanika.clipboard.db`   | Clipboard entries and references to owned image files          |

An empty DB receives its schema atomically. Existing DBs must match the full declared
schema, including indexes, constraints and triggers, and `user_version=1`. Mismatches
are rejected without modifying records. Development data is reset only on request.

- Host: unchanged built-in registration does not rewrite rows. History and usage
  commit together after execution; usage references inventory through a foreign key.
- Application: retain lower-priority sources so removing a preferred root still
  selects the correct winner after restart. Derive normalized display names in
  memory; persist launch metadata and icon extraction source/index. Scan progress
  and resume cursors are not stored.
- Clipboard: a kind-aware content hash provides identity and deduplication. An
  ordering index serves history/retention; a partial image index avoids reading text
  payloads. Capture and configured retention commit together, then publish changed
  and removed entries. Load history once at startup and preserve untouched payloads.
  Later image cleanup failure is reported separately from the successful DB commit.
- Scripts: no durable catalog; rediscover on process start.

Tooling uses empty SQLite files under `target/.build-locks` for exclusive locks.
They contain no product data and are not shipped.

## Application discovery and reconciliation

Application and Script scan on startup and configuration apply. Every native launcher
open requests background refresh from all dynamic Root Search contributors. Windows
and macOS share `show_launcher`/`toggle_launcher`; hiding and DOM focus do not scan.
One queued or active refresh serves repeated opens. Built-in startup discovery also
serves opens received before completion. Existing results/input remain usable.

Refresh replies are dispatched independently of queries/actions. Configuration and
refresh mutations serialize per instance. Each dedicated scanner visits roots
sequentially using streaming recursive enumeration without following directory links.
Filter file types before reading metadata. Windows application roots remain recursive;
macOS `.app` bundles are leaves, so package contents are not traversed. There is no
watcher, polling, F5 command or scan checkpoint. Files changed while open are observed
on the next open or configuration apply.

| Platform | Built-in application sources                                                                                |
| -------- | ----------------------------------------------------------------------------------------------------------- |
| Windows  | Known Folders for Start Menu/packaged apps; SCOOP/SCOOP_GLOBAL or profile/ProgramData roots for Scoop shims |
| macOS    | /Applications, /System/Applications, current HOME/Applications                                              |

Built-in sources default to enabled. Settings shows current-platform switches before
custom folders and preserves other-platform values. Do not embed usernames/drives or
silently replace invalid overrides. Scoop visits shims, not versioned apps/caches.

Windows identity uses canonical target plus arguments; activation keeps the original
shortcut/wrapper. Paired `.shim` metadata can resolve identity, with bounded reads
and concrete malformed/missing-target outcomes. Wrappers with extra environment,
working-directory, elevation or variable semantics stay distinct. Matching names do
not merge distinct targets. macOS identity uses bundle ID or executable path.

Each application root finishes recursive traversal before committing its changed/
removed source rows in one transaction and publishing, then the next root starts.
Unchanged roots produce no write or catalog wake. Failed paths/subtrees and unvisited
roots retain previous records; cancellation discards only current-root staging.
Earlier commits survive process exit. An uninterrupted pass cleans removed/disabled
roots. Unresolved roots restrict cleanup to known roots; resolved failures protect
only their path, respecting directory boundaries and alternate sources. Restart loads
the last committed catalog and scans from the first root.

Scripts publish completed roots from memory. Failed/unvisited roots retain old data.
Configuration application stages the catalog and restores it on failure; rejected
settings never become refresh configuration. Both scanners leave protocol handling
available for catalog batches and host-mediated actions.

Source: [application index](../apps/extensions/built-in/application/src/ApplicationIndex.rs),
[platform adapters](../apps/extensions/built-in/application/src/platform.rs),
[script discovery](../apps/extensions/built-in/script/src/ScriptDiscovery.rs).

## Catalog publication and search

Manifest `contributes.rootSearch.mode` is required. `catalog` supplies query-independent
entries through the Nanika protocol; Application and Script use it. `query` computes
per input; Calculator uses it. External extensions use the same contracts.

`CandidatesChanged` schedules `CatalogRead`. Ordered `CatalogBatch` replies carry
transaction/index, replace/complete flags, upserts and removed IDs. The publisher
targets 256 entries per reply to yield, not to limit catalog size. Host preparation
runs off the search owner; staged changes become visible atomically under the instance
publication gate. `CatalogApplied` follows search-owner commit. Pending publication
is immutable; concurrent changes form the next transaction. Only initial publication
replaces the catalog; later deltas touch affected entries and empty deltas do not rerank.
Actions/configuration retain priority between batches. Query changes do not cancel
catalog transactions; instance retirement withdraws their authority. Query-mode
responses remain generation-bound and do not own discovery cancellation.

The host retains all lightweight entries and the complete ranking in memory. Catalogs
and ranked snapshots share immutable payloads; ranking adds references/scores. Titles,
aliases and compact romanized readings are prepared once per changed entry. Pinyin,
initials and mixed-script matching search the full catalog. Superseded queries cancel
between matching batches and around sorting. There is no top-K or catalog byte ceiling.
DBs support persistence, not per-keystroke search; icons remain references to resources.

The Channel delivers `totalResults`, `resultRevision`, `resultOffset` and a requested
window. `read_results` binds to session/query/result revision; monotonic range IDs
reject reordered requests. One writer coalesces ranges behind one unacknowledged
message and omits unchanged rows on navigation-only updates. The initial 64 rows and
viewport overscan are delivery targets, not total limits. The frontend renders virtual
rows and reconciles only the delivered window; see
[result state](design-system.md#root-search).

Application discovery initially publishes the extension icon without per-entry cache
probes. Visible-range requests prepare cached/native icons in small batches. One
queued/active wake represents the latest viewport. Batch completion rechecks pending
work under the admission lock, preserving requests that arrive during extraction even
when the old batch publishes no changes. Hidden UI schedules no viewport requests.

## Extension image resources

Manifests require a package-relative PNG icon. Commands/views may declare package
icons; candidates accept `{ "kind": "package", "path": "assets/item.png" }` or
`{ "kind": "cache", "key": "file-icon" }`. Omitted icons inherit the extension icon.
The frontend receives resolved URLs through a shared renderer, without domain artwork.

Paths contain slash-separated ASCII letters, digits, dots, underscores and hyphens,
with no empty/dot/parent segments and at most 512 bytes. PNG byte/dimension limits and
canonical resource-root containment apply. Remote URLs, SVG and arbitrary host paths
are rejected. Image failures produce a neutral placeholder, not lifecycle failure.

The resource protocol exposes `/{extensionId}/package/{path}`,
`/{extensionId}/cache/{key}/{size}.png` and `/{extensionId}/payload/{hash}.png`.
Settings reads package images only; launcher also reads cache/payload images. Package
responses are uncached because updates may replace a path; content-addressed responses
are immutable. Disabled extensions retain registered package roots.

External roots come from installed metadata. Built-in exports derive from
`bundle.externalBin` and manifests into `extensions/{extensionId}/assets`. The shell
supplies Tauri's resource directory on both platforms. Windows default-index app icons
share extraction and transparent-bounds normalization with clipboard file icons;
explicit indices use separate extraction. macOS retains its native extraction policy.

## Native presentation and reveal

Native `WindowEvent::Focused(false)` owns launcher hide-on-blur, gated by the user's
setting. DOM activity only controls visuals/menus. The shell synchronizes window and
WebView visibility using WebView2 `IsVisible` on Windows and WKWebView `setHidden` on
macOS. In-WebView menus need no extra native focus manager.

Settings shows after its retained WebView is ready. Close hides/reset navigation while
retaining field edits and accepted operations. No custom window transition or animation
acknowledgement is used. Windows has transparent undecorated surfaces, CSS corners,
custom caption controls and no native shadows. macOS keeps native titlebar controls
and gestures. Maximized content removes CSS rounding.

Permission-checked file reveal runs off the UI thread. Windows selects Shell items
using DOS/UNC spelling at the boundary; canonical filesystem identity is unchanged.
macOS asks NSWorkspace to select file URLs in Finder. Submission success does not
prove the file manager's rendered result; observable failures remain errors.

## Build and validation

Use `just dev` and `just check`, one root package/bun.lock and pinned Bun. Installed
releases need neither Bun nor Node. Sidecar staging derives from `bundle.externalBin`;
inspect packaged files after toolchain/packaging changes.

Dev/build/check/Computer Use share an exclusive compile-through-launch lock; tooling
tests use a separate lock. Failed builds do not publish stale bundles. Dev refuses a
running Nanika instance and an occupied Vite port. macOS tooling cancels/waits for its
process group; Windows runs hidden `taskkill /T /F` and waits. Termination failures
remain errors; tooling policy does not define extension shutdown.

Temporary output lives under `target`, frontend output under
`apps/desktop/frontend/dist`. `just clean` explicitly removes target after builds and
the development app stop. Startup performs no automatic cache eviction.

Validate native flows on each supported platform. Browser fixtures and cross-compilation
do not prove focus, input, DPI or rendering. Compare runtime latency, frame pacing and
memory under equivalent workloads, including 60/120 Hz and hidden idle. Outstanding
acceptance and release work lives in [TODO](tasks.md).
