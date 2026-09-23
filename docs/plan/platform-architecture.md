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

## Configurable application discovery sources

The application extension owns built-in directory selection. Each built-in source has an enabled-by-default configuration property. Settings shows the current platform's built-in directory switches before the custom folder list; persisted configuration retains the other platform's values. Saves must supply all fields visible on the current platform; hidden fields may be omitted but supplied values still require validation. Configuration application triggers the existing transactional rescan. Turning a source off removes its entries after uninterrupted discovery with resolved roots, while entries discovered through other enabled sources remain eligible.

Windows source constants resolve Start Menu directories through Known Folder IDs, packaged application locations relative to known folders, and Scoop shims through `SCOOP`/`SCOOP_GLOBAL` or the current profile/ProgramData locations. Scoop discovery only visits `shims`, never the versioned `apps` or cache directories. Existing application identity and deduplication are unchanged. macOS source constants use the local, system, and user application directories, expanding the user's home dynamically. No username or drive is embedded in source definitions. Unsupported platforms remain rejected by the platform crate.

The configuration schema's platform visibility and presentation order apply equally to built-in and external extensions. They affect Settings presentation only, not authorization or value validation. Missing source directories contribute no entries. Each source resolves independently on both platforms. Resolution and traversal failures are recorded in logs without user-facing scan errors, while other sources continue. Incomplete scans retain prior indexed entries instead of treating failed sources as empty. On Windows, invalid Scoop overrides and Known Folder errors remain concrete per-source failures; no alternate environment path masks them. Repository checks cover the application index and configuration persistence; actual Tauri UI and macOS runtime validation remain pending.

## Pending validation

- Exercise startup enablement, second-instance activation, stale-instance recovery, global shortcuts, native focus, active-monitor placement, and explicit shutdown on physical Windows and macOS systems.
- Verify extension process-tree containment and descendant termination for the native protocol and ACP. Accepted work must finish or report a concrete failure; shutdown must not invent a timeout or recovery policy.
- Verify atomic file replacement failure behavior, package target selection, executable permissions, and diagnostic file opening on each platform.
- Validate Settings window lifecycle, directory dialog ownership and cancellation, host preferences, theme, shortcut recording, and launch-at-login state in the actual application on both platforms.
- Validate native application discovery, activation, icon rendering, cache reuse, clipboard revisions, file thumbnails, and high-DPI output separately on Windows and macOS.

## Future capabilities

Paste-to-foreground requires a host service with one shared authorization and result contract and separate Windows and macOS adapters. Define foreground target selection, native failure reporting, and clipboard ownership before adding a UI action.

Adding another operating system requires an explicit product decision, a complete adapter set, packaging, CI, physical acceptance, and updated release support. No implicit fallback or compatibility layer is planned.

## Application scan reconciliation by path

On Windows and macOS, uninterrupted discovery replaces records in successfully scanned paths even when another path fails. Failed files or subtrees retain their previous records. Missing paths and removed or excluded roots retire their records; newly discovered entries are inserted in the same transaction. Path matching respects directory separators, using the existing platform path normalization. A cancelled scan performs no deletion. If a built-in root cannot be resolved, cleanup is limited to resolved roots because the unknown source cannot be safely assigned a path. Regression tests cover a failed subtree alongside deleted files and a removed root, path boundaries, unknown root resolution, cancellation, and transactional replacement.

## Discovery failure scope and executable wrappers

Resolved discovery paths remain attached to inspection failures. A Windows packaged directory that denies enumeration protects only that directory; it does not prevent cleanup of unrelated disabled or removed roots. Only failures without a resolved path limit reconciliation to known roots. The shared Windows/macOS retention policy is unchanged; macOS source resolution and icon behavior are unchanged.

Windows executable wrappers with a paired .shim file under shims derive identity from the canonical target and prepended raw arguments. Native activation continues through the original wrapper or shortcut. Metadata is read with a 64 KiB limit; malformed metadata reports a concrete failure. Missing targets contribute no entry. Additional environment, working-directory, elevation, or variable-expansion semantics retain the wrapper's distinct identity. Standard argument variants remain distinct. Parsing never scans versioned installation directories, and target changes are picked up on the next scan.
