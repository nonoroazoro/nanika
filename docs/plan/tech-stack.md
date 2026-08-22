# Nanika Technical Stack

Status: current pre-1.0 implementation baseline. Milestone 5 application discovery, indexing, persistence, icon caching, refresh, and host bootstrap are implemented and reviewed on Windows. Before 1.0, measured platform or maintenance problems may justify a change with updated migration and validation notes.

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
| Background work | Standard-library threads and channels | Named owner threads; no async runtime or project-wide pool. |
| Process launch | `std::process::Command` behind platform adapters | Structured arguments by default; explicit shell mode only. |
| Errors | `thiserror` and standard error traits | Typed errors at crate and host boundaries. |
| Diagnostics | `tracing` and `tracing-subscriber` | Structured events and spans with redaction. |
| Benchmarks | `criterion` as a dev dependency | Default features disabled; targets stay outside runtime crates. |
| Extension runtime | Host-supervised child processes | Every extension uses the same protocol and failure boundary. |
| Extension package | ZIP with `.nanika` suffix | `zip` default features disabled; only `deflate-flate2-zlib-rs`. |
| Package integrity | `sha2` | SHA-256 for corruption detection. |

Use the latest mutually compatible stable releases when adding or updating dependencies. Commit `Cargo.lock`. Do not use Git dependencies, wildcard versions, or pre-release versions by default. Review each non-standard-library dependency for necessity, features, transitive cost, maintenance, and platform support.

## Rust workspace policy

Use a virtual Cargo workspace with `resolver = "3"` and Rust 2024 edition. The root `Cargo.toml` is Cargo-required project metadata, not Nanika user configuration. Share package metadata through `workspace.package`, share dependency versions through `workspace.dependencies` only when feature requirements match, and keep platform-specific features local. Inherit `workspace.lints` in every member. Keep one root `Cargo.lock` and one root `target` directory.

Initial layout:

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
extensions/
  nanika-extension-application/
  nanika-extension-fixture/
```

## Host and extension boundary

The host foundation provides UI, window and input handling, scheduling, persistence boundaries, diagnostics, permissions, platform drivers, extension lifecycle, and shared interaction. It does not implement application launch, command execution, script execution, calculation, clipboard history, or agent communication.

Every domain capability is an extension. This follows the relevant VS Code model. There is no first-party capability class.

- `Built-in`: an extension executable shipped with the default Nanika distribution and enabled by default. It cannot be uninstalled because it belongs to that distribution.
- `External`: an extension executable installed from a `.nanika` package.

Both forms use the same capability contract, lifecycle, settings contribution, permissions, host services, process supervisor, and failure policy. Built-in status grants no extra privilege. The bare host has no domain capability. The default distribution enables command, application, script, calculator, and clipboard history extensions.

### Process boundary

Every extension runs as a separate host-supervised child process. The host owns process creation, protocol I/O, cancellation, timeout, restart, shutdown, reaping, and resource budgets. Host APIs never expose host memory, SQLite connections, global configuration, or another extension's state. Built-in packaging never bypasses this boundary. The MVP does not provide an enforceable OS sandbox, so a child process retains the filesystem access of the current user.

Do not load extensions in-process or through Rust dynamic libraries. This is process and failure isolation, not a security sandbox. MVP extensions are trusted native code; enforceable isolation requires a future sandbox decision.

### Shared interaction

The host owns the single input field, input history, query navigation, search aggregation, contextual ranking, and final ordering. Extensions contribute candidates, actions, and settings. They do not own global query navigation or cross-extension ordering.

## UI and interaction

Use a transparent, undecorated, always-on-top overlay. The event-loop thread owns window state, focus, IME, scale-factor changes, and monitor placement. Repaint only for input, state changes, or active animation. Hidden and idle states do not run a continuous render loop.

The initial UI language uses a dark graphite surface, restrained blue-gray secondary text, a single large query field, 8 px spacing rhythm, and no decorative icon dependency. Summon and dismissal use frame-rate-independent smoothstep timelines of 140 ms and 110 ms. Interruption continues from the current value. Active animation requests repaint at up to 120 Hz, while hidden idle state schedules no continuous repaint. Reduced motion snaps directly to the target and is available through `--reduced-motion` until Settings owns it.

The host explicitly enables the selected `wgpu` backend features through its direct dependency because `eframe`'s `wgpu_no_default_features` intentionally leaves backend selection to the application. The Windows smoke test confirmed that at least one native backend is enabled and startup no longer panics.

The MVP includes a minimal host tray or menu bar item:

- Windows notification-area tray icon.
- macOS `NSStatusItem`.
- `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.

The Settings view contains host settings and dynamically contributed settings from every enabled extension. Built-in and external extensions use the same settings schema and validation path. JSONC remains available as an advanced editing path.

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

Use JSONC for human-edited configuration and manifests. Parse through `serde` and `jsonc-parser`, keep CST types private, and convert to typed Rust values at the boundary. UI edits use targeted CST changes, preserve comments and formatting, reparse and validate, then replace files atomically. Each file has a `formatVersion`, ordered migrations, and a last-known-good backup. A failed migration leaves the original file untouched and starts read-only.

The current `nanika-config` boundary implements bootstrap creation, absolute relocatable config roots, typed JSONC parsing, atomic replacement, refreshed last-known-good bootstrap backup, path-preserving backup names, bootstrap recovery, and read-only fallback. Comment-preserving UI mutations remain part of the settings stage.

## SQLite storage

Use one host database and one database per extension. Every database has exactly one writer that owns its connection and transactions. The host storage owner owns `nanika.db`; each extension owns its database through a named owner thread in its process. Connections never cross process boundaries. Extensions own their schema and migration definitions. The current storage crate enforces isolated database paths and migration tables.

`nanika.db` baseline tables:

- `schema_migrations(version, applied_at)`
- `extensions(extension_id, kind, installed_version, active_version, install_path, package_digest, state, health, last_error, updated_at)`
- `input_history(id, normalized_query, display_query, use_count, first_used_at, last_used_at)`
- `usage_stats(extension_id, entry_id, action_id, query_context, execution_count, last_executed_at)`

The current host schema version is 3. Migration 2 adds stable entry identity to `usage_stats` through a transactional table rebuild. Migration 3 adds the usage-retention index.

Application extension baseline tables:

- `schema_migrations(version, applied_at)`
- `scan_state(id, generation, status, started_at, completed_at, last_error)`
- `app_entries(entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key, file_identity, last_seen_at, stale)`

Clipboard extension owns its content, hash, timestamp, pin, retention, and payload fields. Calculator is stateless in the MVP.

Every database uses embedded, ordered, forward-only migrations in a transaction. The host rejects newer or non-contiguous migration histories. Enable `foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, and `busy_timeout=100 ms`. Checkpoint WAL before maintenance and create a consistent snapshot with `VACUUM INTO` before destructive migrations. A failed extension migration disables only that extension and never deletes old data.

## Threads and process execution

Use named owner threads for storage, application discovery, search aggregation, and platform event sources. Runtime configuration and storage initialization stay off the UI thread. The search owner reuses one `nucleo-matcher` instance. Do not create a thread per query, action, or database operation. Fixed extension workers publish typed snapshots and carry generation IDs. Shutdown stops extension workers, storage, search, and platform events in that order.

Only the host process launcher and extension supervisor may create child processes. Extensions submit typed launch descriptors. The default path passes a program and arguments separately and never invokes a shell. Shell mode is explicit, selects the platform interpreter, requires confirmation policy, drains stdout and stderr concurrently, bounds output, applies timeouts, terminates process trees through platform adapters, and always reaps children.

## Platform adapters

### Single instance

Nanika runs one host instance per user session. Windows uses `Local\com.nanika.nanika` through `CreateMutexW`; a blocking platform event thread owns the hidden activation window. macOS holds `nanika.instance.lock` with `flock`; a blocking platform event thread owns the local Unix socket under `<app-data-root>`. Both feed a bounded host activation channel and tolerate the primary's startup handoff race. A second launch sends the request, then exits.

### Global hotkey

Use `global-hotkey` on the event-loop thread. Register only the configured shortcut's press event. Keep media keys outside the MVP. Registration conflicts and failed replacement must preserve the previous working shortcut and produce diagnostics.

### Application discovery

The application extension scans standard platform roots and user-configured roots with `walkdir`. Do not follow symlinks by default. Refresh at startup and on explicit rescan only. Persist generated metadata in the application extension database. Keep filesystem access out of the search hot path.

Windows uses known folders and native `IShellLinkW` resolution. macOS scans `.app` bundles, reads `Info.plist`, and decodes `.icns`. Paths are refreshable metadata; bundle IDs and resolved executable identities provide stable identities.

The application extension runs as its own process with one discovery owner. The host registers it through the universal worker path; explicit refresh is cancellable and stays off the UI thread. Settings require `formatVersion`; only a missing file selects defaults, while read failures preserve the existing index.

Windows discovery resolves every `.lnk` through Shell COM and validates PE targets before indexing. Validation is reused while canonical path, size, and modification time remain unchanged; benchmarks separate cold validation from warm refresh. Identity uses the canonical executable, effective working directory, and typed arguments, so equivalent shortcuts and direct executables deduplicate without merging different launch behavior. Complete scans stale missing entries and remove entries already stale; cancelled, failed, or partial scans preserve unseen data. SQLite commits each generation atomically. Snapshots below 5,000 entries remain complete; larger indexes use query-aware top-k preselection before host ranking.

Searchable entries publish before icon extraction. The recoverable icon cache uses high-resolution metadata keys, retry markers, exact 32 px and 64 px PNG variants, legacy Windows alpha recovery, and a generated fallback. Optional macOS roots do not make a scan partial; bundle executables require executable permissions. The macOS adapter still requires runtime validation on macOS. Application launch remains deferred to the host launch service milestone.

### Clipboard

The clipboard extension captures permitted text, file URI lists, and images. Windows uses native change delivery. macOS polls `NSPasteboard.changeCount` through `clipboard-rs` at a 250 ms default interval. The watcher only sends bounded events; a worker performs capture, normalization, deduplication, retention, and persistence. Clipboard content never enters diagnostics or synchronized configuration.

### Calculator

The calculator extension uses `fend-core` with `evaluate_preview_with_interrupt`. Evaluation runs off the UI thread with input limits, deadlines, and generation cancellation. The MVP context is deterministic and stateless. Successful results copy through the host clipboard service.

### Startup

Windows uses a quoted absolute executable path under the current-user `Run` key. macOS uses `SMAppService.mainAppService` with a minimum supported version of macOS 13. Startup launches Nanika hidden and idle. The operating system remains the source of effective registration state.

## Extension protocol and package

The universal extension protocol uses stdin and stdout with a 4-byte little-endian length prefix, an 8 MiB maximum frame, and a UTF-8 JSON object. JSON here is IPC only, not configuration.

The current implementation provides typed frames, an off-UI-thread registration handshake, generation-aware cancellation, explicit refresh completion, a one-frame receive queue, query and action deadlines, incremental snapshots with an explicit completion flag, bounded stderr capture, restart budgets, automatic process recovery, and orderly shutdown. `invoke` identifies both the selected entry and action. Interrupted queries are safe to retry after restart. Actions are never replayed after an ambiguous crash because that could duplicate side effects. Outstanding actions are bounded to the result queue capacity, so accepted completion messages are not dropped. Successful `result` messages commit contextual usage through the storage owner. Late frames are ignored by request ID and generation. ACP remains a separate future adapter.

Host messages: `initialize`, `query`, `invoke`, `cancel`, `refresh`, `shutdown`.

Extension messages: `initialized`, bounded `snapshot`, `result`, `refreshed`, `error`, and `shutdownAck`.

Every request carries an ID. Query, action, and refresh messages carry a generation. The host drops stale generations, applies a 2-second handshake deadline, bounds action time, captures bounded stderr, and performs graceful shutdown before termination.

External packages are ZIP archives with a `.nanika` suffix:

```text
manifest.jsonc
bin/<target>/<entrypoint>
resources/
README.md
LICENSE
```

The manifest contains `format`, `manifestVersion`, `id`, `version`, `hostApi`, target entrypoints, capabilities, permissions, activation events, and contributions. IDs are lowercase reverse-DNS segments. Package versions use Semantic Versioning. Installed versions are immutable and staged before an atomic rename. Validate safe paths, collisions, size and compression limits, executable permissions, and SHA-256 before activation. MVP installation is an explicit local path or development directory. No marketplace or background download service is included.

ACP remains a future extension protocol. It will reuse the child-process boundary and stdio transport through a separate adapter. ACP messages will not mix with Nanika control frames.

## Performance and validation

Initial targets:

- Warm summon to interactive overlay: P95 at or below 50 ms.
- Input to updated result state: P95 at or below 16.7 ms.
- Stable 60 FPS, with 120 Hz support when available.
- Hidden idle path: no continuous repaint and near-zero CPU.
- No filesystem, SQLite, or blocking extension work on the UI thread.

Measure p50, p95, p99, frame-time variance, dropped frames, CPU, memory, database size, and thread count on fixed representative Windows and macOS machines. Benchmark query delivery, startup, indexing, extension activation, persistence, and rendering separately. Use `criterion` for deterministic in-process benchmarks and platform measurements for launch, focus, and frame pacing. Performance changes require evidence.

## Deferred or rejected

- Electron, Tauri, and webview UI stacks.
- `glow`, `tokio`, `anyhow`, `log`, `rayon`, and a general tray crate.
- Rust dynamic-library extensions, `libloading`, `abi_stable`, `interprocess`, Wasmtime, and WASI for the MVP.
- Extension marketplace, background downloads, cloud sync, generated-data sync, and enforceable sandboxing.
- ACP runtime implementation, file search, URL search, and other post-MVP capabilities.
