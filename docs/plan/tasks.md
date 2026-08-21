# Nanika Tasks

Status: planning only. Do not scaffold or write implementation code until the project owner explicitly says to start.

## Product direction

Nanika is a native, keyboard-driven capability host for Windows 10 and macOS 13 or later. Core supplies only shared infrastructure. Every domain capability is an extension, including the default command, application, script, calculator, and clipboard history capabilities.

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
- [x] Select Rust, `egui` and `eframe` with `wgpu`, `winit`, `global-hotkey`, `nucleo`, SQLite, and the selected platform adapters.
- [ ] Resolve the final Cargo feature graph and commit `Cargo.lock`.
- [ ] Validate overlay focus, transparency, IME, DPI, monitor placement, `wgpu` backends, and hotkey integration on Windows and macOS.
- [ ] Define the visual language, typography, spacing, colors, icons, empty states, and accessibility behavior.
- [ ] Define animation timing, easing, interruption, reduced-motion behavior, and repaint scheduling.
- [ ] Define representative hardware profiles and performance budgets.

### Scaffolding

- [ ] Create the virtual Cargo workspace with resolver 3, Rust 2024 edition, shared metadata, inherited lints, and the package boundaries for the host, Core, universal extension protocol, platform adapters, and extension executables.
- [ ] Add the minimal host binary and event-loop entry point without domain capabilities.
- [ ] Add the shared typed protocol boundary and a minimal extension process fixture.
- [ ] Add Windows and macOS platform adapter modules with explicit unsupported-operation errors where implementation is not ready.
- [ ] Add test and benchmark target layout plus the project formatting, lint, and test commands.
- [ ] Verify that the host starts and exits, an extension process can be supervised, and the workspace passes the baseline checks.

### Core architecture

- [x] Define Core as UI, window and input handling, scheduling, persistence boundaries, diagnostics, permissions, platform drivers, extension lifecycle, and shared interaction.
- [x] Keep all domain capabilities in extensions. Keep shared search aggregation, ranking, and input history in Core.
- [ ] Define typed host boundaries and error categories, redaction, source chaining, and user-facing diagnostics.
- [ ] Define tracing fields, log levels, redaction, bounded queues, rotation, retention, and flush behavior.
- [ ] Define named owner threads for storage, discovery, search, and platform event sources.
- [ ] Define bounded queues, cancellation, generation handling, ordered shutdown, and worker failure recovery.
- [ ] Define the platform boundary for window, hotkey, discovery, launch, startup, tray, and single-instance behavior.

### Extensions

- [x] Require every built-in and external extension to run as a separate host-supervised child process.
- [x] Use one extension contract for capability, lifecycle, settings, permissions, host services, and failure handling.
- [x] Prohibit extension access to host memory, host databases, the global config root, and other extension processes.
- [ ] Define extension and action identities that survive refreshes and updates.
- [ ] Define manifest fields, activation events, contributions, permissions, dependencies, compatibility, and target entrypoints.
- [ ] Define the universal stdio protocol, handshake, bounded frames, generations, cancellation, timeout, and shutdown.
- [ ] Define the extension supervisor, restart and crash recovery, resource budgets, and child reaping.
- [x] Select `.nanika` ZIP packages, `manifest.jsonc`, immutable versions, staging, atomic activation, path validation, and SHA-256 checks.
- [x] Limit MVP installation to explicit local packages or development directories. No marketplace or background download.
- [x] Reserve ACP as a future child-process protocol adapter with separate wire messages.
- [ ] Define extension-owned settings schema, semantic validation, storage, migrations, and reset behavior.
- [ ] Define the default extension implementations and their acceptance tests.

### Application extension

- [ ] Define Windows and macOS application roots, configured roots, exclusions, permission handling, and explicit rescan behavior.
- [ ] Define stable application identity, metadata, executable validation, stale-entry cleanup, and duplicate handling.
- [ ] Define icon extraction, cache keys, fallback icons, and high-DPI variants.
- [ ] Define startup indexing, cancellation, transactional batches, failure recovery, and generated database schema tests.
- [x] Use startup refresh plus explicit rescan. Do not add a filesystem watcher or periodic scan loop in the MVP.

### Search, ranking, and input history

- [ ] Define normalization, case folding, punctuation, whitespace, aliases, and localized names.
- [ ] Define exact, prefix, token, fuzzy, empty-query, and no-result behavior.
- [ ] Define contextual frequency, recency decay, caps, cold start, privacy, retention, and reset.
- [ ] Define deterministic ranking fixtures and tie-breakers.
- [ ] Define input-history navigation, deduplication, ordering, limits, persistence, and current-query preservation.
- [ ] Keep usage writes asynchronous and outside the summon and query hot paths.

### Overlay and platform integration

- [ ] Define summon, focus, selection, Enter, Escape, focus loss, outside click, dismissal, and launch-completion behavior.
- [ ] Define active-monitor placement, multi-monitor, high-DPI, elevated-window, and full-screen behavior.
- [x] Include the minimal host tray or menu bar entry: `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.
- [x] Provide a Settings view for host settings and dynamically contributed settings from every extension. Keep JSONC as an advanced path.
- [ ] Verify Windows hotkey registration, replacement rollback, repeated activation, long holds, idle CPU, and thread behavior.
- [ ] Verify macOS normal shortcut registration without Accessibility or Input Monitoring permission.
- [x] Select Windows current-user Run registration and macOS `SMAppService.mainAppService`.
- [ ] Verify startup status, enable, disable, stale paths, external disablement, rollback, and hidden idle launch.
- [ ] Define typed launch descriptors, structured arguments, explicit interpreters, shell policy, environment, stdio, output limits, timeout, cancellation, process-tree termination, and reaping.
- [ ] Verify GUI, command, script, batch, and macOS bundle launches on both platforms.
- [x] Select single-instance handoff: Windows `CreateMutexW` plus hidden-window activation; macOS `flock` plus a local Unix socket.
- [ ] Test second-launch activation, stale lock recovery, shutdown cleanup, and per-user/session isolation.

### Configuration and storage

- [x] Select `ProjectDirs::from("com", "nanika", "nanika")` and macOS bundle ID `com.nanika.nanika` as the current pre-1.0 identity.
- [x] Separate relocatable user configuration from machine-local bootstrap metadata and generated data.
- [x] Select JSONC for human-edited configuration and manifests, with typed Rust boundaries and CST-preserving edits.
- [ ] Define host and extension settings models, schema validation, path scope, machine overrides, and secret handling.
- [ ] Define bootstrap relocation, directory creation, permissions, malformed-file recovery, and last-known-good behavior.
- [x] Select one `nanika.db` plus one database per extension. Do not sync live SQLite files.
- [x] Record the baseline host and application tables, ownership, pragmas, WAL policy, and migration boundary in `tech-stack.md`.
- [ ] Implement and test ordered forward-only migrations, consistent snapshots, corruption handling, retention, reset, and interrupted scans.
- [ ] Verify all databases remain under `<app-data-root>/databases` and never enter synchronized configuration.
- [ ] Define bounded local logs, diagnostic export, cache cleanup, and generated-data recovery.

### Quality and release

- [ ] Run `cargo fmt`, `cargo clippy`, library and integration tests, and documentation tests.
- [ ] Add domain tests for application identity, search ranking, calculator behavior, clipboard retention, config migration, and storage recovery.
- [ ] Add deterministic `criterion` benchmarks for query delivery, startup, indexing, extension activation, persistence, and rendering preparation.
- [ ] Measure p50, p95, p99, frame-time variance, dropped frames, CPU, memory, database size, and thread count on fixed Windows and macOS machines.
- [ ] Keep timing comparisons advisory on ordinary CI and require evidence before changing the selected stack.
- [ ] Define Windows packaging and signing.
- [ ] Define macOS packaging, signing, and notarization.
- [ ] Define update, rollback, artifact naming, and release checklist.

## Approval gate

- [ ] Review and approve this plan.
- [ ] Confirm the first implementation milestone.
- [ ] Explicitly authorize scaffolding and code implementation.

## Proposed milestones after approval

1. Scaffolding and host foundation.
2. Single instance, universal extension process boundary, configuration, and storage.
3. Global hotkey, overlay, visual language, and animation baseline.
4. Search aggregation, contextual ranking, input history, and fixtures.
5. Windows application extension with discovery, indexing, and persistence.
6. Command, script, calculator, and clipboard history extensions.
7. Settings, startup, and macOS adapters.
8. Performance, packaging, release, and cross-platform acceptance.
9. Post-MVP ACP extension.
