# Nanika Tasks

Status: the pre-1.0 architecture and MVP capabilities are implemented. The remaining work is cross-platform acceptance, release validation, and explicitly deferred cleanup.

## Product baseline

Nanika is a native, keyboard-driven capability host for Windows 10 and macOS 13 or later. The host provides shared infrastructure only. Application search, commands, scripts, calculation, clipboard history, and future domain capabilities run as extensions.

Built-in and external extensions run as independent host-supervised processes with the same lifecycle, settings, permissions, diagnostics, and failure policy. Built-in status grants no additional privilege. A failed extension must not prevent the host or unaffected features from starting.

Shared host, UI, diagnostics, protocol, configuration, storage, search, and extension lifecycle code remains platform-neutral. Native behavior is isolated behind typed Windows and macOS adapters.

## MVP acceptance criteria

- A configurable global hotkey summons a focused overlay on the active monitor.
- Keyboard input, navigation, input history, Enter, Escape, and reduced motion work without a mouse.
- The default distribution includes application search, commands, scripts, calculation, and clipboard history.
- Extensions contribute bounded incremental candidates and actions without host domain registration.
- Stale generations cannot replace newer results, and repeated execution cannot overwhelm lexical relevance.
- Settings are editable through the host UI and advanced JSONC files.
- External extensions can be installed, repaired, updated, enabled, disabled, and removed through `nanika-cli` while the host is stopped.
- A foreground second launch activates the existing host; a background launch exits without disturbing it.
- Configuration is relocatable, while databases, extension artifacts, payloads, caches, and logs remain machine-local.
- User-facing failures name the affected feature and provide a recovery action without exposing internal process, protocol, path, or storage details.
- Operational diagnostics preserve redaction, record distinct extension IDs independently, remain bounded, and support export.
- Hidden idle performs no continuous repaint or polling beyond explicitly designed platform adapters.

## Remaining pre-1.0 work

### Cross-platform acceptance

- [ ] Validate focus, IME, hotkey conflicts, active-monitor placement, mixed DPI, full-screen behavior, elevated foreground windows, and `wgpu` backends on physical Windows and macOS machines.
- [ ] Validate startup enable, disable, repair, approval, stale paths, external disablement, rollback, and hidden idle launch on both platforms.
- [ ] Validate foreground and background second-launch behavior, stale instance recovery, shutdown cleanup, and per-user isolation on both platforms.
- [ ] Validate application, command, script, batch, clipboard, and macOS bundle actions on their supported platforms.
- [ ] Validate diagnostics and feature-specific user messages for missing, incompatible, and failed built-in and external extensions on both platforms.

### Performance and release

- [ ] Add and run the native UI report on a physical Mac.
- [ ] Record P50, P95, P99, frame-time variance, dropped frames, CPU, memory, database size, and thread count on fixed Windows and macOS reference machines.
- [ ] Run current Windows and macOS quality, packaging, checksum, clean-profile, and binary-inventory checks from clean trees.
- [ ] Sign the Windows artifact and sign, notarize, and staple both macOS artifacts with release credentials.
- [ ] Complete clean-profile installation, startup, summon, settings, actions, diagnostics export, update, rollback, and removal acceptance for every release artifact.

### Deferred cleanup

- [ ] Remove `nanika-extension-acp-dummy` before 1.0 packaging.
- [ ] Add settings migrations and reset behavior only when a released format requires compatibility.
- [ ] Define machine overrides and secret handling before a capability requires them.
- [ ] Add maintenance snapshots and corruption recovery before the first post-release destructive database migration.
- [ ] Add captured output and launched-action process-tree cancellation only when a future capability requires them.
