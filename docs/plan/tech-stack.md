# Nanika Technical Stack

Status: current pre-1.0 implementation baseline. Milestone 3 implementation baseline is implemented. Before 1.0, measured platform or maintenance problems may justify a change with updated migration and validation notes.

## Selected baseline

| Area | Selection | Boundary |
| --- | --- | --- |
| Language | Rust stable | Prefer the standard library when it is sufficient. |
| Platforms | Windows 10 and macOS 13 or later | Validate both platforms; keep platform code behind adapters. |
| UI | `egui` through `eframe` | Disable default features. Enable `wgpu_no_default_features`, `default_fonts`, and `accesskit`. No `glow`, persistence, web, or Linux features. |
| Renderer | Direct `wgpu` | Disable default features. Enable `wgsl`, `dx12` and `vulkan` on Windows, `metal` on macOS. |
| Windowing | `winit` through `eframe` | The event-loop thread owns the window. |
| Global hotkey | `global-hotkey` | One configurable normal modifier-and-key shortcut. |
| Fuzzy matching | `nucleo` | High-level worker with a bounded worker count. The search owner owns the matcher. |
| Application paths | `directories` | Resolve roots once through `ProjectDirs`. |
| Directory traversal | `walkdir` | Bounded recursive scans without a general parallel walker. |
| Windows discovery | `windows-sys` and `lnk` | Known folders and Shell Links. |
| macOS discovery | `std::fs`, `plist`, and `icns` | Application bundles, `Info.plist`, and icons. |
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
| Errors | `thiserror` and standard error traits | Typed `AppError` at host boundaries. |
| Diagnostics | `tracing` and `tracing-subscriber` | Structured events and spans with redaction. |
| Benchmarks | `criterion` as a dev dependency | Default features disabled; targets stay outside runtime crates. |
| Extension runtime | Host-supervised child processes | Every extension uses the same protocol and failure boundary. |
| Extension package | ZIP with `.nanika` suffix | `zip` default features disabled; only `deflate-flate2-zlib-rs`. |
| Package integrity | `sha2` | SHA-256 for corruption detection. |

Use the latest mutually compatible stable releases when implementation begins. Commit `Cargo.lock`. Do not use Git dependencies, wildcard versions, or pre-release versions by default. Review each non-standard-library dependency for necessity, features, transitive cost, maintenance, and platform support.

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
extensions/
  nanika-extension-fixture/
```

## Core and extension boundary

Core provides UI, window and input handling, scheduling, persistence boundaries, diagnostics, permissions, platform drivers, extension lifecycle, and shared interaction. Core does not implement application launch, command execution, script execution, calculation, clipboard history, or agent communication.

Every domain capability is an extension. This follows the relevant VS Code model. There is no first-party capability class.

- `Built-in`: an extension executable shipped with the default Nanika distribution and enabled by default. It cannot be uninstalled because it belongs to that distribution.
- `External`: an extension executable installed from a `.nanika` package.

Both forms use the same capability contract, lifecycle, settings contribution, permissions, host services, process supervisor, and failure policy. Built-in status grants no extra privilege. The bare host has no domain capability. The default distribution enables command, application, script, calculator, and clipboard history extensions.

### Process boundary

Every extension runs as a separate host-supervised child process. The host owns process creation, protocol I/O, cancellation, timeout, restart, shutdown, reaping, and resource budgets. An extension cannot access host memory, host SQLite connections, another extension's database, the global config root, or another extension process. It may request only typed host services allowed by its contract and user configuration. Built-in packaging never bypasses this boundary.

Do not load extensions in-process or through Rust dynamic libraries. This is process and failure isolation, not a security sandbox. MVP extensions are trusted native code; enforceable isolation requires a future sandbox decision.

### Shared interaction

The host owns the single input field, input history, query navigation, search aggregation, contextual ranking, and final ordering. Extensions contribute candidates, actions, and settings. They do not own global query navigation or cross-extension ordering.

## UI and interaction

Use a transparent, undecorated, always-on-top overlay. The event-loop thread owns window state, focus, IME, scale-factor changes, and monitor placement. Repaint only for input, state changes, or active animation. Hidden and idle states do not run a continuous render loop.

The initial UI language uses a dark graphite surface, restrained blue-gray secondary text, a single large query field, 8 px spacing rhythm, and no decorative icon dependency. Summon and dismissal use a state-driven smoothstep transition; active animation requests repaint at up to 120 Hz, while hidden idle state schedules no continuous repaint. Reduced motion snaps directly to the target state.

The host explicitly enables the selected `wgpu` backend features through its direct dependency because `eframe`'s `wgpu_no_default_features` intentionally leaves backend selection to the application. The Windows smoke test confirmed that at least one native backend is enabled and startup no longer panics.

The MVP includes a minimal host tray or menu bar item:

- Windows notification-area tray icon.
- macOS `NSStatusItem`.
- `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.

The Settings view contains host settings and dynamically contributed settings from every enabled extension. Built-in and external extensions use the same settings schema and validation path. JSONC remains available as an advanced editing path.

## Search and ranking

The search owner thread owns `nucleo` and final ranking. UI sends queries and receives immutable snapshots. A query generation prevents stale results from replacing newer results.

Ranking order is deterministic:

1. Exact match.
2. Prefix or token match.
3. Fuzzy match above the relevance threshold.
4. Within a tier, bounded query-contextual frequency and recency boosts.
5. Stable identity and alphabetical tie-breakers.

Global popularity must not outrank a clearly better lexical match. Persist usage asynchronously and keep database work out of the per-keystroke path.

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

The current `nanika-config` boundary implements bootstrap creation, relocatable config roots, typed JSONC parsing, atomic replacement, last-known-good recovery, and read-only fallback. Comment-preserving UI mutations remain part of the settings stage.

## SQLite storage

Use one host database and one database per extension. The host storage owner is the only writer and owns connections and transactions. Extensions own their schema and migration definitions. The current storage crate enforces the database path boundary and opens isolated extension databases with independent migration tables.

`nanika.db` baseline tables:

- `schema_migrations(version, applied_at)`
- `extensions(extension_id, kind, installed_version, active_version, install_path, package_digest, state, health, last_error, updated_at)`
- `input_history(id, normalized_query, display_query, use_count, first_used_at, last_used_at)`
- `usage_stats(extension_id, action_id, query_context, execution_count, last_executed_at)`

Application extension baseline tables:

- `schema_migrations(version, applied_at)`
- `scan_state(id, generation, status, started_at, completed_at, last_error)`
- `app_entries(entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key, file_identity, last_seen_at, stale)`

Clipboard extension owns its content, hash, timestamp, pin, retention, and payload fields. Calculator is stateless in the MVP.

Every database uses embedded, ordered, forward-only migrations in a transaction. Enable `foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, and `busy_timeout=100 ms`. Checkpoint WAL before maintenance and create a consistent snapshot with `VACUUM INTO` before destructive migrations. A failed extension migration disables only that extension and never deletes old data.

## Threads and process execution

Use named owner threads for storage, application discovery, search aggregation, and platform event sources. `nucleo` owns its bounded matcher workers. Do not create a thread per query, action, or database operation. Workers publish typed snapshots, check cancellation, and carry generation IDs.

Only the host process launcher and extension supervisor may create child processes. Extensions submit typed launch descriptors. The default path passes a program and arguments separately and never invokes a shell. Shell mode is explicit, selects the platform interpreter, requires confirmation policy, drains stdout and stderr concurrently, bounds output, applies timeouts, terminates process trees through platform adapters, and always reaps children.

## Platform adapters

### Single instance

Nanika runs one host instance per user session. Windows uses `Local\com.nanika.nanika` through `CreateMutexW`; the existing host receives `Activate` through its hidden event window. macOS holds `nanika.instance.lock` with `flock`; the existing host receives `Activate` through a local Unix socket under `<app-data-root>`. A second launch sends the request, then exits. These are narrow platform adapters, not a general IPC runtime.

### Global hotkey

Use `global-hotkey` on the event-loop thread. Register only the configured shortcut's press event. Keep media keys outside the MVP. Registration conflicts and failed replacement must preserve the previous working shortcut and produce diagnostics.

### Application discovery

The application extension scans standard platform roots and user-configured roots with `walkdir`. Do not follow symlinks by default. Refresh at startup and on explicit rescan only. Persist generated metadata in the application extension database. Keep filesystem access out of the search hot path.

Windows uses known-folder and Shell Link adapters. macOS scans `.app` bundles, reads `Info.plist`, and decodes `.icns`. Paths are refreshable metadata; bundle IDs and resolved executable identities provide stable identities.

### Clipboard

The clipboard extension captures permitted text, file URI lists, and images. Windows uses native change delivery. macOS polls `NSPasteboard.changeCount` through `clipboard-rs` at a 250 ms default interval. The watcher only sends bounded events; a worker performs capture, normalization, deduplication, retention, and persistence. Clipboard content never enters diagnostics or synchronized configuration.

### Calculator

The calculator extension uses `fend-core` with `evaluate_preview_with_interrupt`. Evaluation runs off the UI thread with input limits, deadlines, and generation cancellation. The MVP context is deterministic and stateless. Successful results copy through the host clipboard service.

### Startup

Windows uses a quoted absolute executable path under the current-user `Run` key. macOS uses `SMAppService.mainAppService` with a minimum supported version of macOS 13. Startup launches Nanika hidden and idle. The operating system remains the source of effective registration state.

## Extension protocol and package

The universal extension protocol uses stdin and stdout with a 4-byte little-endian length prefix, an 8 MiB maximum frame, and a UTF-8 JSON object. JSON here is IPC only, not configuration.

The current implementation provides typed frames, initialization validation, generation and cancellation fields, bounded receive queues, timeout-aware lifecycle operations, bounded stderr capture, restart budgets, crash recovery, and orderly shutdown. ACP remains a separate future adapter.

Host messages: `initialize`, `query`, `invoke`, `cancel`, `shutdown`.

Extension messages: `initialized`, bounded `snapshot`, `result`, `error`, and `shutdownAck`.

Every request carries an ID. Query and action messages carry a generation. The host drops stale generations, applies a 2-second handshake deadline, bounds action time, captures bounded stderr, and performs graceful shutdown before termination.

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
- `glow`, `tokio`, `anyhow`, `log`, a general project-wide `rayon` pool, and a general tray crate.
- Rust dynamic-library extensions, `libloading`, `abi_stable`, `interprocess`, Wasmtime, and WASI for the MVP.
- Extension marketplace, background downloads, cloud sync, generated-data sync, and enforceable sandboxing.
- ACP runtime implementation, file search, URL search, and other post-MVP capabilities.
