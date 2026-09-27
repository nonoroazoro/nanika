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

The built-in inventory contains Applications, Scripts, Calculator, Clipboard History and System.
Static command contributions and typed process-launch services are shared extension
contracts, independent of the built-in inventory.

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
Root execution binds to the delivered query generation, result revision and originating
instance. A newly admitted query or changed ranked result set invalidates reviewed
targets. Progress, off-query updates, score changes that preserve ordering, and icon-only
updates preserve execution authority. Menus and confirmation
bind to exact result revisions. Root context-menu targets carry that same result revision,
never the transport revision and never a rebased current revision. Ordinary queued view
input retains stable IDs and is revalidated
under the shared operation/invalidation lock. Never retarget stale actions.

The view-input FIFO coalesces only adjacent unsent selections for the same route.
Actions are barriers. Overflow, replaced routes and unavailable targets fail
explicitly. Blocking input waits for RPC completion and the correlated Channel
revision in either order; selection remains nonblocking. The view-input scheduler
owns query drafts and the component renders its value. Newer and invalid drafts
survive older publications; settled input yields to authoritative `search_text`,
and changing routes discards the previous draft. Failed submissions are not retried.
Preserve accepted work, concrete failures and backpressure.

Nanika frames use a 32-bit byte length with no additional application byte quota.
Receivers grow buffers with received bytes, not the claimed header size. Encoding,
allocation, incomplete frames and I/O failures remain errors. Configuration and
catalogs have no aggregate byte quota; script discovery has no entry-count ceiling.
Schema validation, image bounds and bounded queues remain independent constraints.

## Settings operations

Settings submits one property key and value. Rust validates schema and visibility,
merges it into authoritative configuration and reserves one configuration/lifecycle
operation per extension. Other extensions remain independent. Configuration persistence owns only the per-extension
reservation during file I/O; the global registry lock covers snapshot reads and publication. The frontend admits
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

## System actions

System contributes eight static commands with `activation: onDemand`. Search reads
host-owned manifest candidates without starting its process or sending query IPC.
The extension owns command IDs and labels. It invokes the shared
`SystemAction` host service, available to external extensions under the same
instance, parent-request and per-action permission checks. No arbitrary command or
path is accepted by that service.

`system.lock`, `system.sleep`, `system.displays`, `system.logout`, `system.restart`,
`system.shutdown`, `system.trash.open` and `system.trash.empty` each authorize only
their corresponding operation. Permissions are checked before lazily creating one
blocking worker. Its queue holds at most 16 pending requests; overflow is rejected
before admission. It sleeps when idle, drains admitted work on shutdown and never
replays an action after failure or cancellation. Admission returns a single
`SystemActionSubmitted` receipt, meaning the host owns the command, not that the OS
completed it. The extension immediately returns `Dismiss`. Native dispatch failures
are diagnostic logs only; no completion or progress is sent back to the launcher.

| Operation | Windows 10+ | macOS 13+ |
| --- | --- | --- |
| Lock | `LockWorkStation` | Public Control-Command-Q CGEvents; Accessibility permission |
| Sleep | `SetSuspendState` | `/usr/bin/pmset sleepnow` |
| Displays | Asynchronous `SC_MONITORPOWER` broadcast | `/usr/bin/pmset displaysleepnow` |
| Log out / restart / shut down | `ExitWindowsEx`, without force flags | Fixed System Events AppleScripts; Automation permission |
| Open Trash | Shell Recycle Bin namespace | `NSWorkspace` opens the current user's Trash |
| Empty Trash | `SHEmptyRecycleBinW` across drives; Shell owns native progress | Finder's empty operation; Automation permission |

Windows shutdown privilege is enabled only on an impersonating short-lived thread;
thread exit releases it without modifying the host process token. Fixed AppleScripts run
through `/usr/bin/osascript`, whose process owns the scripting main thread. The system
adapter uses `ProcessLauncher` to start interpreters and power utilities without
capturing output or interpreting exit status. Child reaping only releases process
resources; it does not report execution results. No in-process `NSAppleScript`
crosses the main-thread contract. The host bundle declares
its Automation purpose and the Hardened Runtime Apple Events entitlement
`com.apple.security.automation.apple-events`. Native Automation attribution and denial
require macOS acceptance.
The OS owns the operation after dispatch, including any native UI. Nanika does not
query whether Trash was empty or whether a session/power operation completed.

Log out, restart, shut down and empty Trash require the shared revision-bound action
confirmation. All submitted System commands return `Dismiss`. No execution-result
notification or completion state is retained. System has no database or catalog scanner.

## Persistent storage

Host and extensions own separate databases on background workers. `engine/database`
only configures SQLite and admits the current schema. Owners choose transactions;
search and native event loops perform no database I/O. Runtime DBs use STRICT tables,
WAL, foreign keys, NORMAL synchronization and a bounded busy wait. NORMAL preserves
consistency but may lose recent commits after OS/power failure.

| Database                    | Durable data                                                   |
| --------------------------- | -------------------------------------------------------------- |
| `nanika.db`                 | Extension inventory/provenance, input history and action usage |
| `com.nanika.application.db` | Source records keyed by `(source_id, entry_id)`                 |
| `com.nanika.clipboard.db`   | Clipboard entries and references to owned image files          |

An empty DB receives its schema atomically. Existing DBs must match the full declared
schema, including indexes, constraints and triggers, and `user_version=1`. Mismatches
are rejected without modifying records. Development data is reset only on request.

- Host: unchanged built-in registration does not rewrite rows. History and usage
  commit together after the extension action returns; usage references inventory through
  a foreign key. App, Scripts and System record the launch/submission, never the
  eventual outcome of the launched operation.
- Application: retain lower-priority sources so removing a preferred source still
  selects the correct winner after restart. Filesystem roots and native inventories
  share source-based transactions. Derive normalized display names in memory; persist
  launch metadata and a typed icon source: file path/resource index or Windows AUMID/
  full package identity. Scan progress and resume cursors are not stored.
- Clipboard: a kind-aware content hash provides identity and deduplication. An
  ordering index serves history/retention; a partial image index avoids reading text
  payloads. One storage owner holds the connection and an incremental query cache
  of matching IDs and ordering metadata. A changed query streams payloads once; selection, pagination and
  icon scheduling reuse those IDs. Read only current-page summaries and the selected
  original payload. Full history bodies are never resident together. Capture and
  configured retention commit together and update the active query incrementally. Capture,
  retention and clear publish an instance-scoped view change after every successful
  commit. Clear uses the reviewed matching IDs, including off-page entries, without
  deleting newly captured entries absent from the confirmed scope. Later image cleanup failure
  is reported separately from the successful DB commit, without suppressing publication.
  Transactions return removed entry IDs and final ownership only for affected image paths;
  indexed ownership checks preserve shared paths and replacements. Capture does not scan
  unrelated image rows or payload files. Startup alone reconciles all retained paths with
  owned files. That temporary path set is released; runtime resource bookkeeping retains
  only active copy readers and their current committed ownership.
  Copy acquires a storage-owned image lease before returning its content. Retention
  and clear still commit immediately, but reclaim the image only after the final copy
  reader releases it. A recaptured image remains owned by its new row. The caller holds
  the lease through the host-service outcome; response handoff is synchronous and the
  lease borrows the worker so normal shutdown cannot overtake an active reader.
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
| Windows  | Known Folders for Start Menu; PackageManager registered application inventory; SCOOP/SCOOP_GLOBAL or profile/ProgramData roots for Scoop shims |
| macOS    | /Applications, /System/Applications, current HOME/Applications                                              |

Built-in sources default to enabled. Settings shows current-platform switches before
custom folders and preserves other-platform values. Do not embed usernames/drives or
silently replace invalid overrides. Scoop visits shims, not versioned apps/caches.

Windows packaged applications come from the current user's registered packages and
all entries returned by GetAppListEntriesAsync. The OS supplies localized names and
AUMIDs. One packaged-app setting owns this native inventory; protected installation
folders and package manifests are not scanned. A complete inventory commits atomically;
a failed read preserves its previous contribution while filesystem sources continue.
Disabling the source commits an empty inventory even if another source fails.
Native icon identities store the AUMID and full package identity; Shell resolves the
artwork, and a package update changes its cache identity. These are not filesystem paths.
Typed WindowsPackagedApplication host requests validate AUMIDs with the native parser
and use IApplicationActivationManager. Built-in and external callers use the same
process.launch permission and admission contract. macOS rejects these Windows requests.
Native API contracts: [current-user packages](https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.findpackagesforuser),
[application entries](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.package.getapplistentriesasync),
[AUMID parsing](https://learn.microsoft.com/en-us/windows/win32/api/appmodel/nf-appmodel-parseapplicationusermodelid),
[native activation](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iapplicationactivationmanager-activateapplication).

Windows filesystem identity uses canonical target plus arguments; activation keeps the original
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

## Host effects and resource ownership

Extensions own domain behavior, catalogs, history, retention and views. Host services
provide permission-checked native primitives, identically for built-in and external
extensions. Preparing a request does not admit its effect. The runtime rechecks explicit
interruption and originating-instance admission after preparation. Once admitted, the
host owns the stable input and result even if the producer exits.

All extension PNG resources are static images; the shared reader rejects APNG before
pixel decoding. Clipboard preparation reads bytes within the extension payload root,
decodes each row with cancellation checks, and validates terminal chunks before native admission;
queued native writes never reopen a producer pathname. Preparation, queueing and execution
share eight request credits and one PNG credit, preserving a single encoded image's
resident-byte ceiling. Waiting for a credit observes cancellation; there is no expiry.
The existing canvas pixel limit therefore bounds the entire decoded image. Text/profile
metadata is not decompressed during validation; encoded bytes remain unchanged.
Text/files retain their request-owned strings. No history or retention policy
moves into the host.

The native adapter returns a revision causally tied to its own write:

| Platform | Write and receipt contract |
| --- | --- |
| Windows | A message-only owner writes immediate formats, then closes the clipboard to finalize synthesized formats. A second read-only lock verifies ownership and samples the finalized sequence. Lock failure or ownership loss is a concrete receipt error. No foreground activation, delayed renderer or retry. |
| macOS | Prepare NSPasteboardWriting objects, retain the ownership count returned by clearContents, check writeObjects and verify the same ownership. PNG bytes are submitted directly, without decode/re-encode. |

A replacement owner's write cannot become the receipt for Nanika's write. Ownership
loss on either platform is a concrete error. Native validation remains platform-specific; cross-compilation
does not establish OS behavior. References: [OpenClipboard](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openclipboard),
[SetClipboardData](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setclipboarddata),
[NSPasteboard changeCount](https://developer.apple.com/documentation/appkit/nspasteboard/changecount).

Search-related storage accepts writes through one bounded queue. Submission returns an
`StorageCommit` handle that callers may choose to await; it acknowledges only the durable database
transaction. Launcher invocation submits usage and immediately returns its navigation
outcome without awaiting persistence. Initialization explicitly awaits required inventory
writes. The storage owner retains accepted writes even when the receipt is dropped and
drains them before search shutdown. Full queues apply backpressure without dropping work.

The owner sends the durable receipt before admitting the ordered search projection.
Projection failure cannot change a committed write into failure, and storage or projection
errors remain in owner diagnostics rather than action outcomes or search readiness.
A retained diagnostic does not put the launcher into a fatal state; initialization failures
remain fatal. No automatic replay,
additional worker, unbounded queue or completion notification is introduced.

## Catalog publication and search

Manifest `contributes.rootSearch.mode` is required. `catalog` supplies query-independent
entries through the Nanika protocol; Application and Script use it. `query` computes
per input; Calculator uses it. External extensions use the same contracts.

`CandidatesChanged` schedules `CatalogRead`. Ordered `CatalogBatch` replies carry
transaction/index, replace/complete flags, upserts and removed IDs. The publisher
targets 256 entries per reply to yield, not to limit catalog size. Host preparation
runs off the search owner; the search owner accepts staged changes atomically through
an instance-bound publication capability. Retirement removes that registration; old
capabilities cannot publish into or retire a replacement. `CatalogApplied` follows
search-owner commit outside instance/query gates. Pending publication
is immutable; concurrent changes form the next transaction. Only initial publication
replaces the catalog; later deltas touch affected entries and empty deltas do not rerank.
Actions/configuration retain priority between batches. Query changes do not cancel
catalog transactions; instance retirement withdraws their authority. Query-mode
responses remain generation-bound and do not own discovery cancellation.

Each query immediately publishes available static matches. Query contributors publish
independently; `pendingExtensions` identifies contributors without a completed response.
Partial snapshots retain pending status until `complete=true`, including an empty
terminal delta. Same-generation query admission restores pending without removing
available results; cancellation clears pending while retaining published results.
Manifest command/view entries belong to each replacement baseline. Incremental frames
can only alter dynamic identities; they neither reinsert nor remove manifest entries.
SearchOwner assigns execution authority as query generation plus result revision and
retains the contributing instance identities. Runtime, shell delivery, menus and action
confirmation use this semantic authority; pointer sharing is only a memory optimization.
Authority compares ordered visible targets, their originating instances, titles/subtitles
and action invocation metadata. Icons and matching/ranking implementation data are excluded;
changes to them still deliver updated presentation when necessary. Unchanged ranked payloads
reuse their shared allocation. Delivery detects payload changes independently of authority,
so loading an icon cannot cancel an already reviewed action or leave the icon invisible.
Instance maps are shared between snapshots and copied only when registration changes.
Progress-only publications share the immutable ranked results. They neither rerank
candidates nor advance result authority, resend rows, prepare icons or cancel a
reviewed action confirmation.
An empty or slow contributor never gates another contributor's results. Each publication
is immutable and retains current-generation and delivered-revision execution authority.
Pending status does not disable available results and uses no timer or fallback query.

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

Application discovery initially publishes an explicit empty icon without per-entry
cache probes, preserving the fixed icon slot without requesting an image. A separate
single icon worker serves the latest requested viewport while discovery continues;
only discovery writes the application database. Discovery computes icon keys without
owning a cache writer. Cache lookup and extraction accept read-only entry metadata and
return icon references; only the icon worker publishes presentation state. Complete cache hits publish together
before native extraction begins, and each native completion publishes independently
through the existing bounded catalog transport. The worker rechecks the latest viewport
between items; discovery commits wake pending requests for newly available entries.
Completion changes presentation only and must still match the entry's icon key and
typed extraction source. Removed or replaced sources reject stale completions, while unchanged
sources retain prepared icons across metadata updates. Failed extraction keeps its cache set marked incomplete and uses one shared fallback
reference instead of copying fallback files into each failed cache directory. Cache failures remain explicit
and do not turn repeated viewport requests into automatic retries. An admitted refresh
re-admits failed icons, including a failure completing during that refresh; successful
icons remain prepared and offscreen failures wait for a viewport request. Hidden UI schedules
no viewport requests, and the worker sleeps without polling when there is no work.
Shutdown stops admission, finishes an active native call and drains bounded publications
before joining both workers. Windows file sources use per-size native extraction for
32/64/128 px. Registered Windows applications render once at 256 px and derive all
three sizes. macOS renders once at 256 px through NSWorkspace and normalizes each size.

The opt-in `application` `icons` benchmark compares filesystem icon extraction at
separate sizes with single 128/256 px extraction. It does not exercise registered
Windows application identities. Set
`NANIKA_ICON_BENCH_SOURCES` to a JSON array of absolute source paths,
`NANIKA_ICON_BENCH_OUTPUT` to an isolated directory under `target`, and
`NANIKA_ICON_BENCH_MODE` to `separate`, `single128` or `single256`, then run
`cargo bench -p nanika-extension-application --bench icons`. Each process reports ten
rounds including PNG encoding/writes. `NANIKA_ICON_BENCH_HANDSHAKE=1` pauses before
and after work for an external process-memory sampler. Compare modes in alternating
order with the same sources; process-cold measurements do not imply a cold OS cache.
Single-extraction modes remain evaluation candidates for Windows file sources; the
registered-application path already uses one 256 px render.

## Declarative extension views

List snapshots contain at most 500 items with unique section and item identities.
`pagination` carries an informational label and opaque adjacent cursors; `PageChanged`
identifies the list or detail surface. The shell requires the exact delivered view
revision and validates the cursor against the current surface before dispatch.
Clipboard also checks the storage revision before applying a page request. A page replaces the bounded snapshot; it never appends
all preceding pages. Clipboard renders ten history rows per page, preserves selected
identity through captures, and reconciles removed selections after retention.

Plain-text detail pages preserve Unicode and control characters, with at most 16,384
characters per page. Clipboard retains the full original payload for copy and search;
all text remains reachable through adjacent pages. Display bounds neither truncate
stored content nor change capture/retention policy. Search text is plain user data,
including controls, bounded to 4,096 characters. File paths preserve native characters
and reject NUL; display labels escape controls. Extensions validate each proposed view
before committing its protocol revision; failure leaves the accepted state usable.

The clipboard storage owner retains only the active query's matching IDs and ordering
metadata. Committed captures match the changed payload once; retention and clear
remove committed IDs from that collection. Published clear scopes remain immutable.
Changing query text or content type rebuilds the collection from SQLite; ordinary
mutations do not rescan retained payloads. Page authority still tracks data changes.

Clipboard file-icon refreshes revalidate source metadata on their blocking worker.
Both persistent artifacts and in-process outcomes use the same native metadata
identity: Windows creation/write times, size and attributes; macOS device/inode,
size, modification/change timestamps. An unchanged source retains its
success or settled failure. A changed source is acquired on an existing view refresh,
including resume, without polling or retries of unchanged failures.

`ViewsChanged` is a requestless dirty signal for the sending process instance. The host
coalesces one signal per extension and refreshes that instance's current authorized
route. It carries no view ID, so background-view changes cannot overwrite a different
visible view's pending refresh. Resuming a stacked route reads current data.

## Extension image resources

Manifests require a package-relative PNG icon. Commands/views may declare package
icons; candidates accept `{ "kind": "package", "path": "assets/item.png" }` or
`{ "kind": "cache", "key": "file-icon" }`. `{ "kind": "empty" }` reserves a transparent
icon slot without an image URL and suppresses package inheritance. Native images
retain precedence and their existing cache/extraction lifecycle.

Actions may declare a closed `ActionIcon` identity. Root results without an explicit
item icon use the designated action symbol before inheriting the extension icon.
The shell publishes a discriminated image URL or symbol payload for root results.
System uses these symbols. Host-owned Lucide paths are compiled into the frontend,
with no extension markup, remote URLs, icon font or extra runtime dependency.

Paths contain slash-separated ASCII letters, digits, dots, underscores and hyphens,
with no empty/dot/parent segments and at most 512 bytes. PNG byte/dimension limits and
canonical resource-root containment apply. Remote URLs, SVG and arbitrary host paths
are rejected. Image failures produce a neutral placeholder, not lifecycle failure.

Detail images contain only `{ "path": "<hash>.png" }` under the owning extension's
payload root. Inline data URLs are not part of the protocol; CSP remains restricted
to the existing resource origins.

The resource protocol exposes `/{extensionId}/package/{path}`,
`/{extensionId}/cache/{key}/{size}.png` and `/{extensionId}/payload/{hash}.png`.
Settings reads package images only; launcher also reads cache/payload images. Mutable
package URLs are uncached because updates may replace a path. Settings appends a
content fingerprint, which the handler verifies before serving an immutable response.
Content-addressed cache/payload responses are immutable. Disabled extensions retain
registered package roots.

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
