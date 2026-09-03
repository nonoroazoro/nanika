# Nanika Tasks

Status: the implemented baseline is listed first. Every unchecked item is a TODO. Cross-platform acceptance and release validation are not complete.

## Product baseline

Nanika is a native, keyboard-driven capability host for Windows 10 and macOS 13 or later. The host provides shared infrastructure only. Application search, commands, scripts, calculation, clipboard history, and future domain capabilities run as extensions.

Built-in and external extensions run as independent host-supervised processes with the same lifecycle, settings, permissions, diagnostics, and failure policy. Built-in status grants no additional privilege. A failed extension must not prevent the host or unaffected features from starting.

Shared host, UI, diagnostics, protocol, configuration, storage, search, and extension lifecycle code remains platform-neutral. Native behavior is isolated behind typed Windows and macOS adapters.

## Implemented MVP baseline

- A configurable global hotkey toggles a focused overlay on the active monitor.
- The shared input path handles keyboard input, IME preedit and commit, a native primary UI font with a bounded CJK fallback, navigation, input history, Enter, Escape, and reduced motion without a mouse. Root Search and extension filters use shared single-line input sizing and vertical alignment. Physical Windows and macOS acceptance remains a TODO below.
- Root Search waits for the initial snapshot from every ready extension before publishing a new query generation, virtualizes up to 100 results, keeps keyboard selection visible, selects an existing query when reopened, and activates a result by Enter or pointer click.
- Application search uses localized macOS display names while retaining original names as aliases. Application icons are extracted and cached off the UI thread, normalized by visible alpha bounds, and decoded only for visible rows.
- Calculator contributes a result only when the query contains an explicit symbolic or word operator.
- The default distribution includes application search, commands, scripts, calculation, and clipboard history.
- Extensions contribute bounded incremental candidates, static commands, host-rendered views, and typed actions without host domain registration.
- Clipboard history contributes one Root Search command that opens an independent host-rendered List and Split Detail route. Its current action copies the selected content to the clipboard.
- Route-local view search remains responsive during extension round trips, nested routes close cleanly, and every declared action is reachable from the host UI.
- Stale generations cannot replace newer results, and repeated execution cannot overwhelm lexical relevance.
- Settings are editable through the host UI and advanced JSONC files.
- External extensions can be installed, repaired, updated, enabled, disabled, and removed through `nanika-cli` while the host is stopped.
- A foreground second launch activates the existing host; a background launch exits without disturbing it.
- Configuration is relocatable, while databases, extension artifacts, payloads, caches, and logs remain machine-local.
- User-facing failures name the affected feature and provide a recovery action without exposing internal process, protocol, path, or storage details.
- Operational diagnostics preserve redaction, record distinct extension IDs independently, remain bounded, and support export.
- Hidden idle performs no continuous repaint or polling beyond explicitly designed platform adapters.

## Remaining pre-1.0 work (TODO)

### UI foundation

- [ ] Introduce host-owned design tokens and reusable `egui` components for Root Search, result rows, sections, icons, action bars, key hints, List, Split, Detail, and shared states.
- [ ] Rebuild Root Search through the native `egui` component system, including a bounded presentation model for title, subtitle, category, icon, and accessory content. Approve the result from actual Windows and macOS captures and interaction tests.
- [ ] Rebuild extension List, Split, and Detail rendering with the same component system without granting extensions pixel-level styling or arbitrary drawing access.
- [ ] Implement coherent empty, loading, degraded, and diagnostic states.
- [ ] Validate the visual baseline, keyboard behavior, IME, accessibility, high-DPI rendering, reduced motion, latency, and frame pacing on physical Windows and macOS machines.

### Cross-platform acceptance

- [ ] Validate focus, IME, hotkey conflicts, active-monitor placement, mixed DPI, full-screen behavior, elevated foreground windows, and `wgpu` backends on physical Windows and macOS machines.
- [ ] Validate startup enable, disable, repair, approval, stale paths, external disablement, rollback, and hidden idle launch on both platforms.
- [ ] Validate foreground and background second-launch behavior, stale instance recovery, shutdown cleanup, and per-user isolation on both platforms.
- [ ] Validate application, command, script, batch, clipboard, and macOS bundle actions on their supported platforms.
- [ ] Validate diagnostics and feature-specific user messages for missing, incompatible, and failed built-in and external extensions on both platforms.
- [ ] Validate native IME preedit, commit, candidate-window placement, native primary and CJK fallback font selection, caret alignment, and mixed-script visual alignment on physical Windows and macOS machines.

### Performance and release

- [ ] Add and run the native UI report on a physical Mac.
- [ ] Record P50, P95, P99, frame-time variance, dropped frames, CPU, memory, database size, and thread count on fixed Windows and macOS reference machines.
- [ ] Run current Windows and macOS quality, packaging, checksum, clean-profile, and binary-inventory checks from clean trees.
- [ ] Sign the Windows artifact and sign, notarize, and staple both macOS artifacts with release credentials.
- [ ] Complete clean-profile installation, startup, summon, settings, actions, diagnostics export, update, rollback, and removal acceptance for every release artifact.

### Deferred work

- [ ] Remove `nanika-extension-acp-dummy` before 1.0 packaging.
- [ ] Add settings migrations and reset behavior only when a released format requires compatibility.
- [ ] Define machine overrides and secret handling before a capability requires them.
- [ ] Add maintenance snapshots and corruption recovery before the first post-release destructive database migration.
- [ ] Add captured output and launched-action process-tree cancellation only when a future capability requires them.
- [ ] Define and implement a platform-neutral paste-to-foreground host service with Windows and macOS adapters. Clipboard history currently copies the selected item to the clipboard and closes its view.
