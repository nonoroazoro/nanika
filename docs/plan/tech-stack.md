# Nanika Technical Stack

Status: current pre-1.0 implementation baseline. Before 1.0, measured platform or maintenance problems may justify rewriting the baseline with updated validation notes.

## Selected baseline

| Area | Selection | Boundary |
| --- | --- | --- |
| Language | Rust stable | Prefer the standard library when it is sufficient. |
| Platforms | Windows 10 and macOS 13 or later | Validate both platforms; keep platform code behind adapters. |
| UI | `egui` with `egui-winit` | Direct native integration with a bounded CJK-capable system UI font, clipboard, IME, and accessibility. No `glow`, persistence, web, or Linux features. |
| Renderer | `egui-wgpu` with direct `wgpu` backend selection | Disable default features. Enable `wgsl`, `dx12` and `vulkan` on Windows, `metal` on macOS. |
| Windowing | Direct `winit` | The event-loop thread owns visibility, focus, input, and presentation order. |
| Global hotkey | `global-hotkey` | One configurable normal modifier-and-key shortcut. |
| Fuzzy matching | `nucleo-matcher` | One persistent matcher owned by the named search owner thread. |
| Application paths | `directories` | Resolve roots once through `ProjectDirs`. |
| Directory traversal | `walkdir` | Bounded recursive scans without a general parallel walker. |
| Windows discovery | `windows` and `windows-sys` | Typed Shell COM plus direct known-folder, executable, and icon APIs. |
| macOS discovery | `std::fs`, `plist`, and `icns` | Application bundles, `Info.plist`, and icons. |
| Icon cache encoding | `png` | Deterministic RGBA PNG cache variants and fallback icons. |
| Clipboard | `clipboard-rs` | Native Windows monitoring and measured macOS pasteboard polling. |
| Calculator | `fend-core` | Deterministic, interruptible, arbitrary-precision local evaluation. |
| Startup | `windows-registry` and `objc2-service-management` | Current-user Windows Run entry and macOS `SMAppService.mainAppService`. |
| Tray and menu bar | `windows-sys` and `objc2-app-kit` | Native Windows notification icon and macOS `NSStatusItem`; no general tray crate. |
| Single instance | `windows-sys`, `libc`, and standard library sockets | Windows named mutex plus hidden-window activation; macOS `flock` plus a local Unix datagram socket. |
| Serialization | `serde`, `serde_json`, `jsonc-parser` | JSONC only for human-edited files and manifests. Internal APIs use typed Rust values. |
| Extension IDs and versions | `uuid` and `semver` | UUID v4 for opaque IDs and Semantic Versioning for packages. |
| Database | SQLite through `rusqlite` | Default features disabled; only `bundled`. |
| Background work | Standard-library threads and channels | Named owner threads; no project-wide async runtime or pool. |
| Process launch | `std::process::Command` behind platform adapters | Structured arguments by default; explicit shell mode only. |
| Errors | `thiserror` and standard error traits | Typed errors at crate and host boundaries. |
| Diagnostics | `tracing`, `tracing-subscriber`, and `tracing-appender` | Non-blocking local logs with duplicate suppression and a hard byte cap; no content-bearing query or clipboard fields. |
| Benchmarks | `criterion` as a dev dependency | Default features disabled; targets stay outside runtime crates. |
| Extension runtime | Host-supervised child processes | Every extension uses the same lifecycle and failure boundary; a versioned schema selects its wire adapter. |
| ACP | Official `agent-client-protocol` SDK with `async-io`, `async-channel`, `async-process`, `futures`, and `futures-lite` | Stable ACP v1 only. One isolated supervisor thread drives each ACP process; `rustix` terminates the macOS process group. No project-wide executor. |
| Extension package | ZIP with `.nanika` suffix | `zip` default features disabled; only `deflate-flate2-zlib-rs`. |
| Package integrity | `sha2` | SHA-256 for corruption detection. |

Use the latest mutually compatible stable releases when adding or updating dependencies. Commit `Cargo.lock`. Do not use Git dependencies, wildcard versions, or pre-release versions by default. Review each non-standard-library dependency for necessity, features, transitive cost, maintenance, and platform support.

## Rust workspace policy

Use a virtual Cargo workspace with `resolver = "3"` and Rust 2024 edition. The root `Cargo.toml` is Cargo-required project metadata, not Nanika user configuration. Share package metadata through `workspace.package`, share dependency versions through `workspace.dependencies` only when feature requirements match, and keep platform-specific features local. Inherit `workspace.lints` in every member. Keep one root `Cargo.lock` and one root `target` directory.

Workspace layout:

```text
Cargo.toml
crates/
  nanika-core/
  nanika-host/
  nanika-platform/
  nanika-protocol/
  nanika-storage/
  nanika-config/
  nanika-search/
  nanika-extension-package/
  nanika-cli/
extensions/
  nanika-extension-application/
  nanika-extension-command/
  nanika-extension-script/
  nanika-extension-calculator/
  nanika-extension-clipboard/
  nanika-extension-fixture/
  nanika-extension-acp-dummy/
```

## Cross-platform architecture

Shared host, UI, diagnostics, protocol, configuration, storage, search, and extension lifecycle logic is platform-neutral. Platform-specific behavior exists only behind typed adapters in `nanika-platform` or an extension's explicit platform adapter. A shared feature is not complete if it works on only one supported OS.

Every platform adapter contract must preserve the same user-visible semantics, failure boundary, cancellation behavior, and diagnostics shape on Windows and macOS. Platform implementations may use native APIs, but platform details must not leak into shared state or wire protocols. Linux-specific behavior is not an acceptable fallback and must not enter shared paths unless Linux becomes an explicit supported target through a baseline update.

## Host and extension boundary

The host foundation provides UI, window and input handling, scheduling, persistence boundaries, diagnostics, permissions, platform drivers, extension lifecycle, and shared interaction. It exposes typed platform services but contributes no application, command, script, calculator, clipboard history, or agent capability.

Every domain capability is an extension. This follows the relevant VS Code model. There is no first-party capability class.

- `Built-in`: an extension executable shipped with the default Nanika distribution and enabled by default. It cannot be uninstalled because it belongs to that distribution.
- `External`: an extension executable installed from a `.nanika` package.

Both forms use the same capability contract, lifecycle, settings contribution, permissions, host services, process supervisor, and failure policy. Built-in status grants no extra privilege. The bare host has no domain capability. The default distribution enables command, application, script, calculator, and clipboard history extensions.

A development launch must build `nanika-host` and all five built-in extension packages before starting `target/debug/nanika-host`. The host resolves each companion executable next to its own binary. Building or copying the host alone is an invalid development or packaging layout and produces feature-specific startup diagnostics.

### Process boundary

Every extension runs as a separate host-supervised child process. The host owns process creation, protocol I/O, cancellation, timeout, restart, shutdown, reaping, and resource budgets. On Windows, the host creates extension processes suspended, assigns their kill-on-close Job Object, and resumes them only after containment succeeds. On macOS, each extension starts in its own process group. Host APIs never expose host memory, SQLite connections, global configuration, or another extension's state. Built-in packaging never bypasses this boundary. The MVP does not provide an enforceable OS sandbox, so a child process retains the filesystem access of the current user.

Do not load extensions in-process or through Rust dynamic libraries. This is process and failure isolation, not a security sandbox. MVP extensions are trusted native code; enforceable isolation requires a future sandbox decision.

### Shared interaction

The host owns Root Search, including its input field, input history, query navigation, search aggregation, contextual ranking, and final ordering. Extensions may contribute static commands and bounded dynamic candidates to Root Search. They do not control cross-extension ordering.

A command may complete without a view or push a route-local host-rendered view. The extension supplies a bounded declarative `ListView` or `DetailView`; the host owns pixels, typography, accessibility, keyboard behavior, focus, and platform consistency. A list may request the semantic `Plain` or `Split` layout, sections, selection, detail content, filters, pagination, and typed item actions. A standalone detail may declare actions; actions for a detail nested in a list belong to its selected list item. The extension never receives a raw host widget, native handle, or arbitrary drawing surface.

Each pushed view has an extension-scoped ID and monotonic revision. The host serializes operations for that view, validates every replacement document, and keeps local text editing responsive while coalescing pending search text to the latest value. Back closes the active route immediately from the user's perspective. Overlay dismissal closes every extension route in reverse stack order. Nested routes are bounded, and stale updates cannot mutate a different route.

`ViewActionStyle` communicates primary, secondary, or destructive prominence. It does not grant behavior or permission. Every action is rendered by the host and returned to the owning extension as a typed event. Host services such as clipboard writes and process launches remain separately permission checked.

## UI and interaction

Use an undecorated, always-on-top overlay. The primary process starts with only its tray or menu-bar visible. A hidden native window may prewarm the GPU surface, but the event loop shows it only after a complete frame has been presented for hotkey or menu activation. The event-loop thread owns window state, focus, IME, scale-factor changes, and monitor placement. Windows placement uses target-monitor physical pixels; macOS placement uses global AppKit points converted with the current window scale expected by `winit`. Repaint only for input, state changes, or active animation. Hidden and idle states do not run a continuous render loop.

The shared visibility contract is show, hide, and toggle. Windows implements hide through the normal native visibility command. macOS keeps the overlay ordered in with alpha zero and mouse events disabled, briefly orders it out only when needed to release key focus, and immediately orders it back in. This prevents Carbon hotkey events from waiting for a later `winit` event-loop tick while preserving the same user-visible hidden state and near-zero idle work. The macOS behavior remains isolated in `nanika-platform`.

The initial UI language uses a dark graphite surface, restrained blue-gray secondary text, a prominent route-local query field where applicable, 8 px spacing rhythm, and no decorative icon dependency. The current visual and interaction proposal is documented in [Nanika UI Design](../design/ui.md). The host loads one CJK-capable system UI font off the UI thread and delivers it to the UI independently of storage and extension initialization. It is the primary proportional font so Latin and CJK glyphs in normal UI runs share one font face and compatible metrics. It remains a fallback for the monospace family. A shared resolver selects from bounded font candidates supplied by Windows and macOS adapters; startup never scans the complete system font catalog. Font ownership stays in the host design system and is not exposed as an extension styling capability. IME preedit and commit remain handled through the shared `egui-winit` path on Windows and macOS. Summon paints complete content on its first visible frame. Dismissal uses a frame-rate-independent 110 ms smoothstep timeline, and a new summon interrupts it immediately. Active animation requests repaint at up to 120 Hz, while hidden idle state schedules no continuous repaint. Reduced motion snaps directly to the target and is configurable in Settings; `--reduced-motion` remains a runtime override.

Direct integration lets the host keep the native window hidden through presentation and show it only after a complete frame. The host explicitly enables the selected `wgpu` backend features through its direct dependency.

The MVP includes a minimal host tray or menu bar item:

- Windows notification-area tray icon.
- macOS `NSStatusItem`.
- `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.

The Settings view contains host settings and dynamically contributed settings from every enabled extension. Built-in and external extensions use the same settings schema and validation path. JSONC remains available as an advanced editing path.

Settings use a bounded declarative contract with toggle, text, string-list, and record-table controls. The host renders controls but does not interpret domain configuration. Each extension has one editable contribution and at most one request-correlated update in flight. Its draft is locked until validation and atomic JSONC persistence complete. Host controls remain read-only until their runtime owners load. Application path lists and script records use this same contract; extensions without configurable values contribute an empty section. Host settings contain the hotkey and reduced-motion preference. Startup state remains OS-owned.

## Search and ranking

The named search owner thread owns a persistent `nucleo-matcher` instance, aggregation, usage state, final ranking, and generation-tagged snapshots. The UI replaces a coalesced latest-query slot and wakes the bounded owner queue, so saturation cannot drop the current input. Each extension has a fixed protocol worker with a latest-query slot. New queries cancel in-flight work, incremental snapshots wake the UI, and stale request IDs and generations are discarded.

The `nanika-search` crate implements Unicode lowercase, punctuation-separated, whitespace-collapsed exact, prefix, token, fuzzy, empty-query, and alias matching through `nucleo-matcher`. Extensions may supply localized names as aliases. Candidate search values are normalized once when snapshots enter the host. Queries are capped at 4,096 Unicode scalar values. Fuzzy results require 12 score points per normalized query character. Contextual frequency and seven-day recency decay apply only inside the same lexical tier. Ranking uses top-k selection before sorting; snapshots contain at most 100 results. Each extension contribution is capped at 5,000 candidates and deduplicated by extension, entry, and action identity.

Clipboard history contributes one static `Clipboard History` command to Root Search and does not publish retained clipboard entries there. Invoking the command pushes its route-local `ListView` with a split detail pane, bounded local filtering, content-type filtering, selection, pagination, and typed actions. Clipboard content remains local and never enters diagnostics.

Ranking order is deterministic:

1. Exact match.
2. Prefix match.
3. Token-prefix match.
4. Fuzzy match above the relevance cutoff.
5. Within a tier, bounded query-contextual frequency and recency boosts.
6. Fuzzy score, alphabetical title, and stable identity tie-breakers.

Global popularity cannot outrank a better lexical tier. Usage identity is `(extension_id, entry_id, action_id, query_context)`. History and usage keys lowercase and trim input while preserving punctuation significant to commands and scripts. Input history preserves the current draft and remains bounded. The storage owner is authoritative for usage: it commits first, then updates in-memory ranking. Usage is local-only, resettable, retained for 180 days, and capped at the 10,000 most recent contexts. No SQLite work occurs in the summon or per-keystroke path.

## Paths, configuration, and generated data

Use `ProjectDirs::from("com", "nanika", "nanika")` as the current identity. It produces the macOS bundle identifier `com.nanika.nanika`. This is a reasonable pre-1.0 default and may change if implementation evidence warrants it.

`data_local_dir()` and `cache_dir()` are API methods, not literal directory names:

| Root | Windows | macOS |
| --- | --- | --- |
| `<app-data-root>` | `%LOCALAPPDATA%\nanika\nanika\data` | `~/Library/Application Support/com.nanika.nanika` |
| `<cache-root>` | `%LOCALAPPDATA%\nanika\nanika\cache` | `~/Library/Caches/com.nanika.nanika` |

The local layout is:

```text
<app-data-root>/
  bootstrap.jsonc
  profile/
  databases/
    nanika.db
    extensions/<extension-id>.db
  extensions/<extension-id>/<version>/
  backups/config/
  backups/databases/
  logs/
  payloads/<extension-id>/
<cache-root>/
  icons/
  metadata/
```

`bootstrap.jsonc` contains only the effective config-root locator and machine ID. User configuration is under `<effective-config-root>`:

```text
<effective-config-root>/
  nanika.jsonc
  extensions.jsonc
  extensions/<extension-id>/
  machines/<machine-id>/
```

Only this tree is intended for Dropbox or another file-level sync service. Databases, indexes, clipboard content, extension artifacts, logs, backups, and caches are machine-local generated data and are never synchronized. Never synchronize live SQLite files.

Cleanup follows data ownership. A complete application scan prunes unreferenced icon cache entries and reports cleanup failures through the refresh boundary. The application index runs `quick_check` at startup and rebuilds once when open, load, or scan detects SQLite corruption. Incompatible schemas and access failures remain errors. Clipboard retention removes unreferenced managed payloads. Recovery never deletes `nanika.db`, clipboard history, configuration, installed extension artifacts, or logs.

Use JSONC for human-edited configuration and manifests. Parse through `serde` and `jsonc-parser`, keep CST types private, and convert to typed Rust values at the boundary. UI edits use targeted CST changes, preserve comments and formatting, reparse and validate, then replace files atomically. Each settings file has a `formatVersion`. Before the first release, format changes rewrite the baseline. Released formats require ordered migrations, and a failed migration must leave the original file untouched and start read-only.

The current `nanika-config` boundary implements bootstrap creation, absolute relocatable config roots, typed JSONC parsing, comment-preserving top-level and nested-object changes, atomic replacement, path-preserving backups, bootstrap recovery, and read-only fallback. The extension registry selects defaults only for an explicit missing file; other metadata and access failures remain errors.

## Diagnostics

The host uses stable diagnostic codes and categories. `HostDiagnostic` keeps a safe user message separate from a cloneable technical source chain retained by active UI error state. Operational logs contain only the code, category, operation, and explicitly safe context such as a validated extension ID. Independent extension failures carry distinct safe contexts so duplicate suppression does not merge failures from different extensions. Raw worker errors never enter the UI. Debug formatting redacts messages and sources. Query text, clipboard content, settings values, extension payloads, and external error text are never logged.

User-visible diagnostics describe the unavailable capability and a concrete recovery action without exposing internal extension, process, protocol, or storage terminology. Identical user messages are rendered once even when several independent technical failures share the same recovery path. The underlying diagnostics remain separate in memory and in operational logs.

The primary host is the only log owner. It writes non-blocking INFO-and-higher lifecycle and failure events under `<app-data-root>/logs`. The queue is lossy and bounded to 256 lines. Identical code, operation, context, and level combinations are recorded at most once per 30 seconds, with at most 256 active keys. Startup removes the oldest owned logs above 32 MiB, and the writer stops at the remaining byte budget. Logs rotate daily, retain at most eight files, and flush during orderly shutdown.

`nanika-cli diagnostics <output.zip>` exports version and platform metadata plus at most eight Nanika-owned daily log files and 32 MiB. It rejects symlinks and unrelated files, enforces the byte limit while copying, publishes through a same-directory atomic no-clobber hard link, and can run while the host is active.

## SQLite storage

Use one host database and one database per extension. Every database has exactly one writer that owns its connection and transactions. The host storage owner owns `nanika.db`; each extension owns its database through a named owner thread in its process. Connections never cross process boundaries. Extensions resolve their database path from the host-supplied data root and own their schema and migration definitions.

`nanika.db` baseline tables:

- `schema_migrations(version, applied_at)`
- `extensions(extension_id, kind, installed_version, active_version, install_path, package_digest, state, health, last_error, updated_at)`
- `input_history(id, normalized_query, display_query, use_count, first_used_at, last_used_at)`
- `usage_stats(extension_id, entry_id, action_id, query_context, execution_count, last_executed_at)`

The host, application, and clipboard schemas each start from the current baseline version 1. Host usage identity includes extension, entry, action, and query context, with an index for retention cleanup. Startup parses and validates extension metadata one row at a time, reports malformed rows, and continues with valid extensions.

Application extension baseline tables:

- `schema_migrations(version, applied_at)`
- `scan_state(id, generation, status, started_at, completed_at, last_error)`
- `app_entries(entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key, file_identity, last_seen_at, stale)`

Clipboard extension owns its content, hash, timestamp, pin, retention, and payload fields. Calculator is stateless in the MVP.

The clipboard schema stores typed text, file-list, or image references with content hashes, byte size, capture and last-use timestamps, and pin state. Unpinned history is retained for 30 days and capped at 500 entries. Image PNG payloads live under `<app-data-root>/payloads/com.nanika.clipboard`, outside synchronized configuration and SQLite.

Every database uses an embedded baseline and an ordered forward-only migration runner. Before the first release, schema changes rewrite version 1 and old development databases must be reset. Released schemas increment versions transactionally; the host rejects newer or non-contiguous histories. Enable `foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, and `busy_timeout=100 ms`. Before a post-release destructive migration, checkpoint WAL and create a consistent snapshot with `VACUUM INTO`. A failed extension migration disables only that extension and never deletes old data.

## Threads and process execution

Use named owner threads for storage, application discovery, search aggregation, and platform event sources. Runtime configuration and storage initialization stay off the UI thread. The search owner reuses one `nucleo-matcher` instance. Do not create a thread per query, action, or database operation. Fixed extension workers publish typed snapshots and carry generation IDs. Each active ACP extension owns one additional named supervisor thread for its isolated async protocol executor. Shutdown stops extension workers, storage, search, and platform events in that order.

Only the host process launcher and extension supervisor may create child processes. Extensions submit typed `Program`, `Shell`, or `MacApplication` descriptors. `Program` keeps structured arguments separate, with an explicit Windows raw-argument representation for Shell Links. `Shell` selects `cmd.exe` on Windows and `/bin/zsh` on macOS. MVP launches are detached with null stdio, and action success means the process was accepted. One bounded launcher owner serializes spawn work. Windows releases detached process handles; macOS reaps children through `kqueue` `NOTE_EXIT` events without polling. Captured execution and launched-action process-tree cancellation remain a later descriptor mode.

## Platform adapters

### Single instance

Nanika runs one host instance per user session. Windows uses `Local\com.nanika.nanika` through `CreateMutexW`; a blocking platform event thread owns the hidden activation window and notification icon. macOS holds `nanika.instance.lock` with `flock`; a blocking platform event thread owns a local Unix datagram socket under `<app-data-root>`. One-byte activation and stop datagrams cannot leave the listener blocked on a partial stream connection. Both adapters feed bounded platform events to the host and tolerate the primary's startup handoff race. A foreground second launch requests activation, then exits. A background second launch exits without activation.

### Global hotkey

Use `global-hotkey` on the event-loop thread. The default shortcut is `Ctrl+Space` on macOS and `Alt+Space` on Windows. A press toggles the launcher when it owns the activation and ensures the launcher opens for second-instance activation. A press during dismissal interrupts the animation and reveals the existing surface immediately. Keep media keys outside the MVP. Registration conflicts and failed replacement must preserve the previous working shortcut and produce diagnostics.

Measure native hotkey delivery through a passive `nanika-platform` observer before `global-hotkey` discards the source timestamp. The observer must always continue native event propagation and must never become an alternate hotkey delivery path. Use Carbon `EventTime` on macOS and `MSG.time` on Windows, then expose only a platform-neutral `Duration` to the host. Missing native timing must mark a sample incomplete instead of silently treating callback time as input time.

### Application discovery

The application extension scans standard platform roots and user-configured roots with `walkdir`. Do not follow symlinks by default. Refresh at startup and on explicit rescan only. Persist generated metadata in the application extension database. Keep filesystem access out of the search hot path.

Windows uses known folders and native `IShellLinkW` resolution. macOS scans `.app` bundles, reads `Info.plist`, and decodes `.icns`. Paths are refreshable metadata; bundle IDs and resolved executable identities provide stable identities.

The application extension runs as its own process with one discovery owner. The host registers it through the universal worker path; explicit refresh is cancellable and stays off the UI thread. Settings require `formatVersion`; only a missing file selects defaults, while read failures preserve the existing index.

Windows discovery resolves every `.lnk` through Shell COM and validates PE targets before indexing. Validation is reused while canonical path, size, and modification time remain unchanged; benchmarks separate cold validation from warm refresh. Identity uses the canonical executable, effective working directory, and typed arguments, so equivalent shortcuts and direct executables deduplicate without merging different launch behavior. Complete scans stale missing entries and remove entries already stale; cancelled, failed, or partial scans preserve unseen data. SQLite commits each generation atomically. Snapshots below 5,000 entries remain complete; larger indexes use query-aware top-k preselection before host ranking.

Searchable entries publish before icon extraction. The recoverable icon cache uses high-resolution metadata keys, retry markers, exact 32 px and 64 px PNG variants, legacy Windows alpha recovery, and a generated fallback. Complete scans prune unreferenced cache entries; extraction failures increase scan warnings and prune failures fail the refresh. SQLite corruption or a non-database file rebuilds only the derived application index, with one retry; incompatible schemas and access failures remain visible errors. Optional macOS roots do not make a scan partial; bundle executables require executable permissions. Application actions now submit persisted typed launch metadata to the common host service. The macOS adapter still requires runtime validation on macOS.

### Clipboard

The clipboard extension captures permitted text, file lists, and images. Windows uses native change delivery. macOS polls `NSPasteboard.changeCount` through `clipboard-rs` at a 250 ms interval. The watcher only sends bounded events; one owner performs capture, deduplication, retention, payload cleanup, and SQLite persistence. Oversized content is skipped, never truncated: text and encoded file lists are limited to 1 MiB, file lists to 256 paths, and PNG images to 16 MiB, 8,192 pixels per dimension, and 16,777,216 pixels. Explicit refresh completes only after capture and persistence; worker errors are reported through the protocol. The implemented action uses the common host clipboard service, is labeled `Copy to Clipboard`, and closes the view after a successful copy. TODO: define a separate platform-neutral paste-to-foreground host service with Windows and macOS adapters before presenting paste behavior. Clipboard content never enters diagnostics or synchronized configuration.

### Calculator

The calculator extension uses `fend-core` with `evaluate_preview_with_interrupt`. Evaluation runs in its extension process with a 4,096-character input limit and a 50 ms interrupt deadline. The MVP context is deterministic and stateless. Successful results copy through the common host clipboard service.

### Command and script

The command extension contributes only for queries beginning with `>` and submits the remaining text as an explicit `Shell` descriptor. The script extension loads stable entries from `extensions/com.nanika.script/settings.jsonc`; every entry names an absolute interpreter, script path, structured arguments, and optional working directory. A missing script settings file means an empty contribution. Neither extension creates child processes directly.

### Startup

Windows uses a quoted absolute executable path under the current-user `Run` key. macOS uses `SMAppService.mainAppService` with a minimum supported version of macOS 13. Startup launches Nanika hidden and idle. The operating system remains the source of effective registration state.

Startup status and mutations run through a bounded platform owner and report their effective state back to the host. Windows treats an unexpected existing Run value as needing repair. macOS preserves `RequiresApproval` and `NotFound` instead of collapsing them into a Boolean; approval opens Login Items rather than repeating registration.

The native tray or menu bar emits only `Open Nanika`, `Settings`, `Rescan applications`, and `Quit` events. Windows owns its notification icon on the existing hidden-window thread and negotiates notification callback version 4. macOS creates `NSStatusItem` on the AppKit main thread. Neither adapter owns domain behavior.

## Extension protocol and package

Nanika protocol v1 uses stdin and stdout with a 4-byte little-endian length prefix, an 8 MiB maximum frame, and a UTF-8 JSON object. ACP v1 uses its standard newline-delimited JSON-RPC 2.0 stdio transport with an 8 MiB frame limit in both directions. The two wire protocols never share a stream.

`ExtensionRuntime` is the common supervisor entry for built-in and external extensions. It selects the wire adapter from validated runtime metadata without changing permissions, lifecycle, or failure policy. Invocation outcomes are `Completed`, `Cancelled`, or `Failed`; only completion records usage, and cancellation is not shown as a failure. Failure recovery uses a fixed restart budget. User cancellation may relaunch a non-cooperative extension without consuming that budget; shutdown cancellation never relaunches it.

The Nanika adapter provides typed frames, an off-UI-thread registration handshake, generation-aware cancellation, explicit refresh completion, a one-frame receive queue, query, action, and settings deadlines, incremental snapshots with an explicit completion flag, bounded stderr capture, restart budgets, automatic process recovery, request-correlated extension settings results, and orderly shutdown. `invoke` identifies both the selected entry and action. Interrupted queries are safe to retry after restart. Actions and settings updates are never replayed after an ambiguous crash. Outstanding actions are bounded to the result queue capacity, so accepted completion messages are not dropped. Successful `result` messages commit contextual usage through the storage owner. Late frames are ignored by request ID and generation.

The ACP adapter negotiates stable v1, creates one session, and contributes a prompt candidate only for `@<extension-id> <prompt>`. It streams text off the UI thread, limits stderr to 64 KiB and prompt output to 256 KiB, and uses the common handshake and action deadlines. Escape or dismissal cancels the active invocation. Cancellation first sends the ACP notification; timeout or non-cooperation terminates the process tree. User cancellation relaunches the extension without consuming its failure budget, while shutdown does not relaunch it. Each invocation has a unique host ID. Workers publish bounded protocol-neutral delta batches, and the UI accepts only current output while laying out at most the latest 16 KiB. ACP extensions contribute empty settings and receive no ACP client capabilities or Nanika host-service privilege by default.

Host messages: `initialize`, `query`, `invoke`, `cancel`, `refresh`, `getSettings`, `updateSettings`, `hostResponse`, `error`, and `shutdown`.

Extension messages: `initialized`, bounded `snapshot`, `result`, `refreshed`, `settings`, `settingsUpdated`, `hostRequest`, `error`, and `shutdownAck`.

Each `hostRequest` is bound to its parent invocation and generation, and extensions validate matching `hostResponse` fields. The same router handles built-in and external extensions. Service owners have independent bounded queues and independent initialization failure, so one unavailable service does not disable the others. Host service waits remain inside the parent action deadline, and queued work expires before starting a side effect. Current services accept typed launch descriptors and typed clipboard writes; image writes are confined to the requesting extension's machine-local payload root and are read with encoded and decoded resource limits.

Nanika requests carry IDs; query, action, and refresh messages also carry a generation. ACP uses its standard JSON-RPC request and session IDs, plus a host invocation ID for output correlation. The host drops stale generations, applies a 2-second handshake deadline, bounds action time and stderr, performs orderly normal shutdown, and force-terminates a process tree after deadline or failed cancellation. Windows uses hidden suspended creation and binds a kill-on-close Job Object before the initial thread runs; macOS uses a dedicated process group and propagates termination failures other than an already-missing group.

External packages are ZIP archives with a `.nanika` suffix:

```text
manifest.jsonc
bin/<target>/<entrypoint>
resources/
README.md
LICENSE
```

Manifest version 1 requires `runtime: { protocol, protocolVersion }`; current values are Nanika v1 and ACP v1. Unknown protocols, versions, fields, targets, unsupported or duplicate permissions, and unsafe entrypoints are rejected. IDs and dependency IDs are lowercase reverse-DNS segments, and package versions use Semantic Versioning. The MVP supports `process.launch` and `clipboard.write`. Nanika extensions may declare bounded static commands and Root Search participation through `contributions`; ACP extensions retain their existing prompt activation path. Capabilities, dependencies, and activation events remain reserved.

`nanika-cli` installs, updates, enables, disables, and removes external extensions while the host is stopped. `install` creates a missing extension or repairs the same immutable version; a different installed version requires `update`. `update` requires an installed extension, preserves enablement, and rejects downgrades. Archives are limited to 128 MiB, 4,096 entries, and 512 MiB expanded content. Traversal, symlinks, cross-platform name collisions, filesystem collisions, unsupported compression, and excessive compression ratios are rejected. Extraction never overwrites an existing path. The package is copied and hashed once, then extraction uses that immutable staged copy. The target entrypoint is made executable on macOS. Destructive artifact mutations write a recovery journal before rename; host startup and later CLI operations finish or roll back an interrupted replacement or removal. Configuration, database state, and artifacts use ordered mutations with compensation, and generated cleanup is best-effort after logical commit. Built-in IDs cannot be replaced or removed. No marketplace, development-directory install, or background download service is included.

The temporary `nanika-extension-acp-dummy` is an ordinary workspace extension that implements stable ACP v1 and returns `Hello World`. Tests package it as `.nanika`, install and resolve it through the normal external-extension path, publish its explicitly activated prompt candidate, and verify streamed output through the host adapter. Protocol tests also verify v1 negotiation, unique session IDs, bounded frames and deadlines, repeated cancellation, shutdown without relaunch, reusable recovery, startup containment, and descendant termination. It has no runtime privilege, is excluded from release packaging, and must be removed before 1.0.

## Performance and validation

Targets:

- Warm summon to interactive overlay: P95 at or below 50 ms.
- Input to updated result state: P95 at or below 16.7 ms.
- Stable 60 FPS, with 120 Hz support when available.
- Hidden idle path: no continuous repaint and near-zero CPU.
- No filesystem, SQLite, or blocking extension work on the UI thread.

Measure p50, p95, p99, frame-time variance, dropped frames, CPU, memory, database size, and thread count on fixed representative Windows and macOS machines. Benchmark query delivery, startup, indexing, extension activation, persistence, and rendering separately. Use `criterion` for deterministic in-process benchmarks and platform measurements for launch, focus, and frame pacing. Performance changes require evidence.

Release builds use thin LTO and one codegen unit. Windows ships a signed portable x86-64 ZIP. macOS ships a Developer ID signed, hardened, notarized, and stapled `.app` ZIP for Apple silicon or Intel. Every artifact is immutable, versioned, and paired with SHA-256. The MVP has no installer or updater framework; update and rollback replace the complete stopped application from a verified artifact while preserving external user data.

## Deferred or rejected

- Electron, Tauri, and webview UI stacks.
- Direct host use of `glow`, `tokio`, `anyhow`, `log`, `rayon`, and a general tray crate.
- Rust dynamic-library extensions, `libloading`, `abi_stable`, `interprocess`, Wasmtime, and WASI for the MVP.
- Extension marketplace, background downloads, cloud sync, generated-data sync, and enforceable sandboxing.
- Draft ACP v2, production agent UX, file search, URL search, and other later capabilities.
