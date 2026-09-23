# Platform Architecture: Open Work

Implemented platform mechanisms live in `engine/platform` and the Tauri shell. Only macOS 13+ and Windows 10+ are supported. This document records the remaining contract and validation gates, not a second description of adapter source code.

## Shared contract

A platform adapter may change native mechanisms, not product semantics, lifecycle, cancellation, failure behavior, or protocol shapes. Shared engine and frontend code must not select native APIs or carry native handles. Unsupported targets fail explicitly at the adapter boundary.

For each new platform-facing behavior, record:

1. The shared contract and ownership.
2. The Windows implementation.
3. The macOS implementation.
4. The unsupported-platform result.
5. Runtime evidence on both supported platforms.

Cross-target compilation is not native runtime validation.

## Build tooling ownership

Invocations reuse `target/cargo` with Cargo's default incremental compilation.
Each command has a fixed staging directory. Dev, build, check, and Computer Use
share one exclusive SQLite lock for the complete compile, stage, package, and launch
pipeline. Cargo's internal lock alone cannot protect the steps after compilation.
The OS releases the lock on process death. Tooling tests can run independently.
Build failures stop the pipeline; only freshly generated bundles are published.
Development refuses an already running Nanika instance instead of activating it.
The script owns Vite and awaits its listener before passing that exact URL to Tauri;
an occupied frontend port fails instead of reusing a previous server.

Cleanup is manual: `just clean` recursively removes the entire repository `target`,
including compiler caches, bundles, and lock files. Stop builds and the development
app first. Startup performs no size scans, capacity checks, or automatic eviction.

On macOS, tooling signals the command's process group and waits for it to stop.
On Windows, cancellation uses `taskkill /T /F`. Unsupported platforms fail before
build work begins. Native Windows execution remains a required validation gate.

## Pending validation

- Exercise startup enablement, second-instance activation, stale-instance recovery, global shortcuts, native focus, active-monitor placement, and explicit shutdown on physical Windows and macOS systems.
- Verify extension process-tree containment and descendant termination for the native protocol and ACP. Accepted work must finish or report a concrete failure; shutdown must not invent a timeout or recovery policy.
- Verify atomic file replacement failure behavior, package target selection, executable permissions, and diagnostic file opening on each platform.
- Validate Settings window lifecycle, directory dialog ownership and cancellation, host preferences, theme, shortcut recording, and launch-at-login state in the actual application on both platforms.
- Validate native application discovery, activation, icon rendering, cache reuse, clipboard revisions, file thumbnails, and high-DPI output separately on Windows and macOS.

## Future capabilities

Paste-to-foreground requires a host service with one shared authorization and result contract and separate Windows and macOS adapters. Define foreground target selection, native failure reporting, and clipboard ownership before adding a UI action.

Adding another operating system requires an explicit product decision, a complete adapter set, packaging, CI, physical acceptance, and updated release support. No implicit fallback or compatibility layer is planned.
