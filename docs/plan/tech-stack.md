# Nanika Technical Stack

Status: current pre-1.0 implementation baseline. Milestone 8 physical acceptance and removal of the temporary ACP dummy remain. Milestone 9 is complete. Before 1.0, measured platform or maintenance problems may justify rewriting the baseline with updated validation notes.

## Selected baseline

| Area | Selection | Boundary |
| --- | --- | --- |
| Language | Rust stable | Prefer the standard library when it is sufficient. |
| Platforms | Windows 10 and macOS 13 or later | Validate both platforms; keep platform code behind adapters. |
| UI | `egui` through `eframe` | Disable default features. Enable `wgpu_no_default_features`, `default_fonts`, and `accesskit`. No `glow`, persistence, web, or Linux features. |
| Renderer | Direct `wgpu` | Disable default features. Enable `wgsl`, `dx12` and `vulkan` on Windows, `metal` on macOS. |
| Windowing | `winit` through `eframe` | The event-loop thread owns the window. |
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
| Single instance | `windows-sys`, `libc`, and standard library sockets | Windows named mutex plus hidden-window activation; macOS `flock` plus a local Unix socket. |
| Serialization | `serde`, `serde_json`, `jsonc-parser` | JSONC only for human-edited files and manifests. Internal APIs use typed Rust values. |
| Extension IDs and versions | `uuid` and `semver` | UUID v4 for opaque IDs and Semantic Versioning for packages. |
| Database | SQLite through `rusqlite` | Default features disabled; only `bundled`. |
| Background work | Standard-library threads and channels | Named owner threads; no project-wide async runtime or pool. |
| Process launch | `std::process::Command` behind platform adapters | Structured arguments by default; explicit shell mode only. |
| Errors | `thiserror` and standard error traits | Typed errors at crate and host boundaries. |
| Diagnostics | `tracing`, `tracing-subscriber`, and `tracing-appender` | Bounded non-blocking local logs; no content-bearing query or clipboard fields. |
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

## Host and extension boundary

The host foundation provides UI, window and input handling, scheduling, persistence boundaries, diagnostics, permissions, platform drivers, extension lifecycle, and shared interaction. It exposes typed platform services but contributes no application, command, script, calculator, clipboard history, or agent capability.

Every domain capability is an extension. This follows the relevant VS Code model. There is no first-party capability class.

- `Built-in`: an extension executable shipped with the default Nanika distribution and enabled by default. It cannot be uninstalled because it belongs to that distribution.
- `External`: an extension executable installed from a `.nanika` package.

Both forms use the same capability contract, lifecycle, settings contribution, permissions, host services, process supervisor, and failure policy. Built-in status grants no extra privilege. The bare host has no domain capability. The default distribution enables command, application, script, calculator, and clipboard history extensions.

### Process boundary

Every extension runs as a separate host-supervised child process. The host owns process creation, protocol I/O, cancellation, timeout, restart, shutdown, reaping, and resource budgets. On Windows, the host creates extension processes suspended, assigns their kill-on-close Job Object, and resumes them only after containment succeeds. On macOS, each extension starts in its own process group. Host APIs never expose host memory, SQLite connections, global configuration, or another extension's state. Built-in packaging never bypasses this boundary. The MVP does not provide an enforceable OS sandbox, so a child process retains the filesystem access of the current user.

Do not load extensions in-process or through Rust dynamic libraries. This is process and failure isolation, not a security sandbox. MVP extensions are trusted native code; enforceable isolation requires a future sandbox decision.

### Shared interaction

The host owns the single input field, input history, query navigation, search aggregation, contextual ranking, and final ordering. Extensions contribute candidates, actions, and settings. They do not own global query navigation or cross-extension ordering.

## UI and interaction

Use a transparent, undecorated, always-on-top overlay. The event-loop thread owns window state, focus, IME, scale-factor changes, and monitor placement. Windows placement uses target-monitor physical pixels; macOS placement uses global AppKit points converted with the current window scale expected by `winit`. Repaint only for input, state changes, or active animation. Hidden and idle states do not run a continuous render loop.

The initial UI language uses a dark graphite surface, restrained blue-gray secondary text, a single large query field, 8 px spacing rhythm, and no decorative icon dependency. Summon and dismissal use frame-rate-independent smoothstep timelines of 140 ms and 110 ms. Interruption continues from the current value. Active animation requests repaint at up to 120 Hz, while hidden idle state schedules no continuous repaint. Reduced motion snaps directly to the target and is configurable in Settings; `--reduced-motion` remains a runtime override.

The host explicitly enables the selected `wgpu` backend features through its direct dependency because `eframe`'s `wgpu_no_default_features` intentionally leaves backend selection to the application. The Windows smoke test confirmed that at least one native backend is enabled and startup no longer panics.

The MVP includes a minimal host tray or menu bar item:

- Windows notification-area tray icon.
- macOS `NSStatusItem`.
- `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.

The Settings view contains host settings and dynamically contributed settings from every enabled extension. Built-in and external extensions use the same settings schema and validation path. JSONC remains available as an advanced editing path.

Settings use a bounded declarative contract with toggle, text, string-list, and record-table controls. The host renders controls but does not interpret domain configuration. Each extension has one editable contribution and at most one request-correlated update in flight. Its draft is locked until validation and atomic JSONC persistence complete. Host controls remain read-only until their runtime owners load. Application path lists and script records use this same contract; extensions without configurable values contribute an empty section. Host settings contain the hotkey and reduced-motion preference. Startup state remains OS-owned.

## Search and ranking

The named search owner thread owns a persistent `nucleo-matcher` instance, aggregation, usage state, final ranking, and generation-tagged snapshots. The UI replaces a coalesced latest-query slot and wakes the bounded owner queue, so saturation cannot drop the current input. Each extension has a fixed protocol worker with a latest-query slot. New queries cancel in-flight work, incremental snapshots wake the UI, and stale request IDs and generations are discarded.

The `nanika-search` crate implements Unicode lowercase, punctuation-separated, whitespace-collapsed exact, prefix, token, fuzzy, empty-query, and alias matching through `nucleo-matcher`. Extensions may supply localized names as aliases. Candidate search values are normalized once when snapshots enter the host. Queries are capped at 4,096 Unicode scalar values. Fuzzy results require 12 score points per normalized query character. Contextual frequency and seven-day recency decay apply only inside the same lexical tier. Ranking uses top-k selection before sorting; snapshots contain at most 100 results. Each extension contribution is capped at 5,000 candidates and deduplicated by extension, entry, and action identity.

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

Use JSONC for human-edited configuration and manifests. Parse through `serde` and `jsonc-parser`, keep CST types private, and convert to typed Rust values at the boundary. UI edits use targeted CST changes, preserve comments and formatting, reparse and validate, then replace files atomically. Each settings file has a `formatVersion`. Before the first release, format changes rewrite the baseline. Released formats require ordered migrations, and a failed migration must leave the original file untouched and start read-only.

The current `nanika-config` boundary implements bootstrap creation, absolute relocatable config roots, typed JSONC parsing, comment-preserving top-level and nested-object changes, atomic replacement, path-preserving backups, bootstrap recovery, and read-only fallback. The extension registry selects defaults only for an explicit missing file; other metadata and access failures remain errors.

## Diagnostics

The host writes non-blocking INFO-and-higher lifecycle and fatal events under `<app-data-root>/logs`. The queue is lossy and bounded to 256 lines so diagnostics cannot stall the UI. Logs rotate daily, retain at most eight files, and flush during orderly shutdown. Query text, clipboard content, settings values, and extension payloads are not logged.

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

Nanika runs one host instance per user session. Windows uses `Local\com.nanika.nanika` through `CreateMutexW`; a blocking platform event thread owns the hidden activation window and notification icon. macOS holds `nanika.instance.lock` with `flock`; a blocking platform event thread owns the local Unix socket under `<app-data-root>`. Both feed bounded platform events to the host and tolerate the primary's startup handoff race. A foreground second launch requests activation, then exits. A background second launch exits without activation.

### Global hotkey

Use `global-hotkey` on the event-loop thread. Register only the configured shortcut's press event. Keep media keys outside the MVP. Registration conflicts and failed replacement must preserve the previous working shortcut and produce diagnostics.

### Application discovery

The application extension scans standard platform roots and user-configured roots with `walkdir`. Do not follow symlinks by default. Refresh at startup and on explicit rescan only. Persist generated metadata in the application extension database. Keep filesystem access out of the search hot path.

Windows uses known folders and native `IShellLinkW` resolution. macOS scans `.app` bundles, reads `Info.plist`, and decodes `.icns`. Paths are refreshable metadata; bundle IDs and resolved executable identities provide stable identities.

The application extension runs as its own process with one discovery owner. The host registers it through the universal worker path; explicit refresh is cancellable and stays off the UI thread. Settings require `formatVersion`; only a missing file selects defaults, while read failures preserve the existing index.

Windows discovery resolves every `.lnk` through Shell COM and validates PE targets before indexing. Validation is reused while canonical path, size, and modification time remain unchanged; benchmarks separate cold validation from warm refresh. Identity uses the canonical executable, effective working directory, and typed arguments, so equivalent shortcuts and direct executables deduplicate without merging different launch behavior. Complete scans stale missing entries and remove entries already stale; cancelled, failed, or partial scans preserve unseen data. SQLite commits each generation atomically. Snapshots below 5,000 entries remain complete; larger indexes use query-aware top-k preselection before host ranking.

Searchable entries publish before icon extraction. The recoverable icon cache uses high-resolution metadata keys, retry markers, exact 32 px and 64 px PNG variants, legacy Windows alpha recovery, and a generated fallback. Optional macOS roots do not make a scan partial; bundle executables require executable permissions. Application actions now submit persisted typed launch metadata to the common host service. The macOS adapter still requires runtime validation on macOS.

### Clipboard

The clipboard extension captures permitted text, file lists, and images. Windows uses native change delivery. macOS polls `NSPasteboard.changeCount` through `clipboard-rs` at a 250 ms interval. The watcher only sends bounded events; one owner performs capture, deduplication, retention, payload cleanup, and SQLite persistence. Oversized content is skipped, never truncated: text and encoded file lists are limited to 1 MiB, file lists to 256 paths, and PNG images to 16 MiB, 8,192 pixels per dimension, and 16,777,216 pixels. Explicit refresh completes only after capture and persistence; worker errors are reported through the protocol. Restore uses the common host clipboard service. Clipboard content never enters diagnostics or synchronized configuration.

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

Manifest version 1 requires `runtime: { protocol, protocolVersion }`; current values are Nanika v1 and ACP v1. Unknown protocols, versions, fields, targets, unsupported or duplicate permissions, and unsafe entrypoints are rejected. IDs and dependency IDs are lowercase reverse-DNS segments, and package versions use Semantic Versioning. The MVP supports `process.launch` and `clipboard.write`. Capabilities, dependencies, activation events, and manifest contributions remain reserved.

`nanika-cli` installs, updates, enables, disables, and removes external extensions while the host is stopped. `install` creates a missing extension or repairs the same immutable version; a different installed version requires `update`. `update` requires an installed extension, preserves enablement, and rejects downgrades. Archives are limited to 128 MiB, 4,096 entries, and 512 MiB expanded content. Traversal, symlinks, cross-platform name collisions, filesystem collisions, unsupported compression, and excessive compression ratios are rejected. Extraction never overwrites an existing path. The package is copied and hashed once, then extraction uses that immutable staged copy. The target entrypoint is made executable on macOS. Destructive artifact mutations write a recovery journal before rename; host startup and later CLI operations finish or roll back an interrupted replacement or removal. Configuration, database state, and artifacts use ordered mutations with compensation, and generated cleanup is best-effort after logical commit. Built-in IDs cannot be replaced or removed. No marketplace, development-directory install, or background download service is included.

The temporary `nanika-extension-acp-dummy` is an ordinary workspace extension that implements stable ACP v1 and returns `Hello World`. Tests package it as `.nanika`, install and resolve it through the normal external-extension path, publish its explicitly activated prompt candidate, and verify streamed output through the host adapter. Protocol tests also verify v1 negotiation, unique session IDs, bounded frames and deadlines, repeated cancellation, shutdown without relaunch, reusable recovery, startup containment, and descendant termination. It has no runtime privilege, is excluded from release packaging, and must be removed before 1.0.

## Performance and validation

Initial targets:

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
