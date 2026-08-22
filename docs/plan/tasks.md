# Nanika Tasks

Status: implementation in progress. Milestone 6 is complete and post-review fixes are verified on Windows. The default capabilities run as built-in extension processes through common search, action, host service, deadline, and failure boundaries. The macOS platform crate also passes cross-target checks.

## Product direction

Nanika is a native, keyboard-driven capability host for Windows 10 and macOS 13 or later. The host foundation supplies only shared infrastructure. Every domain capability is an extension, including the default command, application, script, calculator, and clipboard history capabilities.

Built-in extensions are shipped and enabled by default. External extensions are installed separately. Both run as independent host-supervised processes and use the same contract, settings, permissions, and failure policy. Built-in status grants no privilege and only prevents uninstall from the default distribution.

## MVP acceptance criteria

- Configurable global hotkey summons an overlay on the active monitor.
- Keyboard input, navigation, input history, Enter, and Escape work without a mouse.
- Default extensions provide command execution, application execution, script execution, calculation, and clipboard history.
- Extensions can contribute searchable entries and actions without host registration.
- Query results appear incrementally and stale generations cannot replace newer results.
- Repeated execution raises the matching action for the same query context without overwhelming lexical relevance.
- Settings are editable in the host Settings view and through advanced JSONC editing.
- Built-in and external extension settings are loaded dynamically through the same contribution contract.
- A user can install, enable, update, disable, and remove external extensions.
- Missing, incompatible, or failed extensions do not prevent host startup.
- A second Nanika launch activates the existing host instead of creating another UI or hotkey registration.
- Startup integration works on Windows 10 and macOS 13 or later.
- User configuration is in one relocatable tree; generated data remains machine-local.
- Startup, indexing, search, launch, and permission failures produce actionable diagnostics.

## TODO

### Baseline and quality

- [x] Record the selected stack and rejected directions in `tech-stack.md`.
- [x] Select Rust, `egui` and `eframe` with `wgpu`, `winit`, `global-hotkey`, `nucleo-matcher`, SQLite, and the selected platform adapters.
- [ ] Resolve the final Cargo feature graph and commit `Cargo.lock`.
- [ ] Validate overlay focus, transparency, IME, DPI, monitor placement, `wgpu` backends, and hotkey integration on Windows and macOS.
- [x] Define the visual language, typography, spacing, colors, icons, empty states, and accessibility behavior.
- [x] Define animation timing, easing, interruption, reduced-motion behavior, and repaint scheduling.
- [ ] Define representative hardware profiles and performance budgets.

### Scaffolding

- [x] Create the virtual Cargo workspace with resolver 3, Rust 2024 edition, shared metadata, inherited lints, and the package boundaries for the host, shared Core types, protocol, platform adapters, storage, and extension executables.
- [x] Add the minimal host binary and event-loop entry point without domain capabilities.
- [x] Add the shared typed protocol boundary and a minimal extension process fixture.
- [x] Add Windows and macOS platform adapter modules with explicit unsupported-platform errors.
- [x] Add test and benchmark target layout plus the project formatting, lint, and test commands.
- [x] Verify that the host and fixture start and exit, and that the workspace passes the baseline checks.

### Host architecture

- [x] Define the host foundation as UI, window and input handling, scheduling, persistence boundaries, diagnostics, permissions, platform drivers, extension lifecycle, and shared interaction.
- [x] Keep all domain capabilities in extensions. Keep shared search aggregation, ranking, and input history in the host foundation.
- [ ] Define typed host boundaries and error categories, redaction, source chaining, and user-facing diagnostics.
- [ ] Define tracing fields, log levels, redaction, bounded queues, rotation, retention, and flush behavior.
- [ ] Complete named owner threads for storage, discovery, search, and platform event sources.
- [ ] Complete bounded queues, cancellation, generation handling, ordered shutdown, and worker failure recovery across all MVP capabilities.
- [ ] Complete platform adapters for window, hotkey, discovery, launch, startup, tray, and single-instance behavior.

### Extensions

- [x] Require every built-in and external extension to run as a separate host-supervised child process.
- [ ] Define one extension contract for capability, lifecycle, settings, permissions, host services, and failure handling.
- [x] Keep host APIs and IPC from exposing host memory, SQLite connections, global configuration, or another extension's state. The MVP does not claim an enforceable filesystem sandbox.
- [x] Implement the off-UI-thread registration handshake, bounded JSON frames, and orderly shutdown fixture.
- [x] Define extension and action identities that survive refreshes and updates.
- [x] Define manifest fields, activation events, contributions, permissions, dependencies, compatibility, and target entrypoints.
- [x] Define the universal stdio protocol, handshake, bounded frames, generations, cancellation, timeout, and shutdown.
- [x] Implement bounded protocol queues, deadlines, restart budgets, safe query retry, non-replayed action recovery, graceful shutdown, and child reaping.
- [x] Select `.nanika` ZIP packages, `manifest.jsonc`, immutable versions, staging, atomic activation, path validation, and SHA-256 checks.
- [x] Limit MVP installation to explicit local packages or development directories. No marketplace or background download.
- [x] Reserve ACP as a future child-process protocol adapter with separate wire messages.
- [ ] Define extension-owned settings schema, semantic validation, storage, migrations, and reset behavior.
- [x] Define the default extension implementations and their acceptance tests.

### Application extension

- [x] Define Windows and macOS application roots, configured roots, exclusions, permission handling, and explicit rescan behavior.
- [x] Define stable application identity, metadata, executable validation, stale-entry cleanup, and duplicate handling.
- [x] Define icon extraction, cache keys, fallback icons, and high-DPI variants.
- [x] Define startup indexing, cancellation, transactional batches, failure recovery, and generated database schema tests.
- [x] Use startup refresh plus explicit rescan. Do not add a filesystem watcher or periodic scan loop in the MVP.

### Search, ranking, and input history

- [x] Define Unicode lowercase, punctuation, whitespace, and identity-preserving behavior.
- [x] Implement consistent aliases, including localized names supplied as aliases.
- [x] Define exact, prefix, token, fuzzy, and empty-query behavior.
- [x] Implement the fuzzy relevance cutoff and no-result policy.
- [x] Define and implement contextual frequency, recency decay, caps, cold start, local-only privacy, 180-day retention, and reset.
- [x] Align usage identity with the persisted extension, entry, action, and query-context schema.
- [x] Define deterministic ranking fixtures and tie-breakers.
- [x] Implement bounded in-memory input-history navigation, deduplication, ordering, and limits.
- [x] Preserve the draft query during navigation and persist history.
- [x] Integrate empty-query and incremental search snapshots with the host and extension contributions.
- [x] Coalesce query bursts without dropping the latest input or accepting stale results.
- [x] Bound outstanding actions so accepted completion messages are never dropped.
- [x] Recover from timeout and ignore late protocol frames without poisoning later requests.
- [x] Make committed storage usage authoritative for in-memory ranking.
- [x] Keep usage writes asynchronous and outside the summon and query hot paths.

### Overlay and platform integration

- [x] Define summon, focus, selection, Enter, Escape, and dismissal behavior for the initial overlay.
- [ ] Define active-monitor placement, multi-monitor, high-DPI, elevated-window, and full-screen behavior.
- [ ] Include the minimal host tray or menu bar entry: `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.
- [ ] Provide a Settings view for host settings and dynamically contributed settings from every extension. Keep JSONC as an advanced path.
- [x] Implement Windows hotkey registration, replacement rollback, repeated activation handling, and event-loop delivery.
- [x] Implement the macOS normal shortcut registration boundary. Runtime permission validation remains.
- [x] Select Windows current-user Run registration and macOS `SMAppService.mainAppService`.
- [ ] Verify startup status, enable, disable, stale paths, external disablement, rollback, and hidden idle launch.
- [ ] Define typed launch descriptors, structured arguments, explicit interpreters, shell policy, environment, stdio, output limits, timeout, cancellation, process-tree termination, and reaping.
- [ ] Verify GUI, command, script, batch, and macOS bundle launches on both platforms.
- [x] Select single-instance handoff: Windows `CreateMutexW` plus hidden-window activation; macOS `flock` plus a local Unix socket.
- [x] Integrate startup-race-safe Windows hidden-window and macOS lock/socket activation with the host through bounded blocking event sources.
- [ ] Test second-launch activation, stale lock recovery, shutdown cleanup, and per-user/session isolation.

### Configuration and storage

- [x] Select `ProjectDirs::from("com", "nanika", "nanika")` and macOS bundle ID `com.nanika.nanika` as the current pre-1.0 identity.
- [x] Separate relocatable user configuration from machine-local bootstrap metadata and generated data.
- [x] Select JSONC for human-edited configuration and manifests, with typed Rust boundaries and CST-preserving edits.
- [ ] Define host and extension settings models, schema validation, path scope, machine overrides, and secret handling.
- [x] Implement bootstrap relocation, directory creation, malformed-file recovery, and last-known-good read-only fallback.
- [x] Select one `nanika.db` plus one database per extension. Do not sync live SQLite files.
- [x] Record the baseline host and application tables, ownership, pragmas, WAL policy, and migration boundary in `tech-stack.md`.
- [x] Implement the resolved path boundary, host migration runner, and isolated extension database owner.
- [ ] Implement and test ordered forward-only migrations, consistent snapshots, corruption handling, retention, reset, and interrupted scans.
- [x] Verify all databases remain under `<app-data-root>/databases` and never enter synchronized configuration.
- [ ] Define bounded local logs, diagnostic export, cache cleanup, and generated-data recovery.

### Quality and release

- [x] Run `cargo fmt`, `cargo clippy`, library and integration tests, and documentation tests.
- [x] Add domain tests for application identity, search ranking, calculator behavior, clipboard retention, config migration, and storage recovery.
- [ ] Add deterministic `criterion` benchmarks for query delivery, startup, indexing, extension activation, persistence, and rendering preparation.
- [ ] Measure p50, p95, p99, frame-time variance, dropped frames, CPU, memory, database size, and thread count on fixed Windows and macOS machines.
- [ ] Keep timing comparisons advisory on ordinary CI and require evidence before changing the selected stack.
- [ ] Define Windows packaging and signing.
- [ ] Define macOS packaging, signing, and notarization.
- [ ] Define update, rollback, artifact naming, and release checklist.

## Approval gate

- [x] Review and approve this plan.
- [x] Confirm the first implementation milestone.
- [x] Explicitly authorize scaffolding and code implementation.

## Implementation milestones

1. [x] Scaffolding and host foundation.
2. [x] Single instance, universal extension process boundary, configuration, and storage.
3. [x] Global hotkey, overlay, visual language, and animation baseline.
4. [x] Search aggregation, contextual ranking, input history, and fixtures.
5. [x] Windows application extension with discovery, indexing, and persistence.
6. [x] Command, script, calculator, and clipboard history extensions.
7. Settings, startup, and macOS adapters.
8. Performance, packaging, release, and cross-platform acceptance.
9. Post-MVP ACP extension.
