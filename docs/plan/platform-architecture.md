# Platform Architecture Baseline

## Supported targets

Nanika's current supported and future release targets are only:

- macOS 13 or later
- Windows 10 or later

Linux and every other platform are explicitly unsupported. They must not consume a macOS or Windows implementation through an implicit fallback.

## Design model

Nanika uses product-level cross-platform support. Shared layers define stable contracts and product semantics. Each supported target may provide a native implementation behind a typed adapter when that produces better behavior or user experience.

```text
shared contract
    -> platform-neutral service interface
        -> macOS adapter or Windows adapter
            -> target-specific Tauri wiring and release artifact
```

Shared code includes protocol DTOs, runtime orchestration, domain behavior, storage schemas, search, diagnostics schemas, frontend components, and user-visible semantics. It must not contain native handles, operating-system paths, platform APIs, or platform conditionals.

Storage follows the same boundary. Shared storage contracts define records, transactions, limits, and failure semantics. The host owns only host data and each extension owns only its own database and payloads through its process-local storage owner. Database schemas are current pre-release baselines, so an incorrect schema is rewritten directly; no migration or compatibility reader is retained.

The extension protocol is platform-neutral and versioned independently from packaging targets. It carries typed bounded data and actions, never native paths, handles, platform identifiers, or frontend code. Runtime supervision, operation ordering, backpressure, cancellation, and lifecycle are shared behaviors. A platform adapter may change the mechanism used to provide them, but not the protocol or product semantics.

Extension configuration follows the same boundary. The extension's validated `contributes.configuration` contribution declares the static schema, the host owns JSONC persistence and validates complete effective snapshots, and the extension consumes only the typed snapshot delivered by its protocol adapter. Nanika extensions acknowledge a live update by request ID only after their configuration-dependent work is complete; accepting work into a queue is not an acknowledgement. ACP receives the effective snapshot in `session/new` metadata and has no live-update acknowledgement. Configuration keys, values, acknowledgements, and errors remain platform-neutral.

The frontend is one shared Svelte application for both release targets. Tauri exposes the same typed commands and channel contracts on macOS and Windows. Target-specific window, tray, shortcut, process, clipboard, and startup behavior stays in the shell wiring or platform adapter and is excluded from shared presentation state. The current shell creates only the launcher WebView window; the Settings window and its shell wiring remain unimplemented.

Platform adapters own native APIs, handles, event sources, process containment, window placement, startup registration, single-instance activation, clipboard monitoring, and other target-specific mechanisms. Adapters expose the same contract, failure boundary, cancellation behavior, lifecycle, and diagnostics shape on macOS and Windows.

Clipboard writes return an opaque native revision after the write completes. The Clipboard extension gates watcher delivery while its own write is in flight, filters revisions produced by that write, and captures only a later external revision. macOS reads `NSPasteboard.changeCount` and compares its native-width wrapping counter. Windows reads `GetClipboardSequenceNumber` and compares the wrapping 32-bit sequence. Shared protocol and extension code treat the revision as an opaque ordering token; no native handle or API type crosses the adapter boundary. Unsupported operating systems continue to fail at the platform crate boundary.

`engine/platform` owns the adapter contracts and implementations. Its top-level module only selects the implementation for the build target and wires it to the shared contract. Tauri shell code owns only unavoidable application and window wiring. No extension, protocol, runtime, frontend, or domain module selects a platform implementation.

## Process and filesystem contracts

System file icons are a shared `engine/platform` capability. macOS uses `NSWorkspace.iconForFile` and sRGB drawing. On Windows, Application discovery uses [`IShellItemImageFactory::GetImage`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellitemimagefactory-getimage) with `SIIGBF_ICONONLY`, followed by `SHGetFileInfoW` and `ExtractIconExW` when needed. Clipboard uses the same image factory without `SIIGBF_ICONONLY`, so Explorer returns a content thumbnail when available and otherwise returns the native file or directory icon. Failed thumbnail acquisition falls back to the 256 px `SHIL_JUMBO` system image list and then the direct `SHGetFileInfoW` icon. Windows bitmap dimensions are read with `GetObjectW`. Shell PARGB bitmaps and alpha-bearing BGRA icon drawings are unpremultiplied into straight RGBA; dual-background alpha recovery remains inside the Windows adapter for legacy icons without alpha. File icon requests run in the extension process, never on the WebView or Tauri event loop. Clipboard passes only captured file paths to this service. Missing or inaccessible file icons retain the existing semantic file artwork; they never discard clipboard entries or block copying file paths.

The shared file-icon cache draws at 512 px and writes immutable, metadata-keyed 128 and 512 px PNGs under the requesting extension's icon cache. Its identity covers the render version, absolute native path, and platform metadata stamp, so complete variants are reused across extension process restarts without invoking the native icon service. List rows use the 128 px variant at 26 CSS px; file details use the 512 px variant for a larger preview. Declarative views carry validated opaque icon references for image loading. File paths may appear as bounded display-only text; they grant no filesystem access and are never used as image URLs. Windows runtime verification remains pending on this macOS host.

Application icons use the operating system's resolved application image, then share one pixel-processing contract: crop only fully transparent outer margins and fit the remaining artwork to the cache square without extra inset. Preserve aspect ratio, opaque black and white areas, partial alpha, shadows, and every pixel with nonzero alpha when finding crop bounds. Neither platform infers backgrounds from color or adds its own icon tile or mask.

- macOS 13+: resolve the application bundle with `NSWorkspace.iconForFile`, then draw the returned `NSImage` once into a transparent 256 px sRGB Core Graphics bitmap. Convert premultiplied RGBA to straight alpha before normalization. Generate all missing 32, 64, and 128 px PNG variants from that working image; raw bundle image decoding and TIFF conversion are not acquisition paths. The OS controls the icon's appearance; macOS 26's automatic corner and plate treatment is not promised on older macOS versions.
- Windows 10+: retain Shell item image acquisition at each requested cache size and resource selection. Convert premultiplied native pixels to straight RGBA, recover alpha for legacy icons when required, then apply the same normalization and cache publication rules.
- Shared presentation: publish searchable metadata before background icon population, reuse complete persistent caches, and display the 128 px variant in a 26 CSS px slot without additional padding, background, or mask. Preserve system-supplied visual treatment within the pixels. Incomplete caches and failed extraction use the documented placeholder policy; system-provided generic icons are valid native results.

The macOS cache identity covers rendering version, OS version, bundle metadata, executable metadata, and icon-resource metadata, including custom-icon changes. It does not call the native icon service while discovering searchable metadata. Windows application identities remain unchanged. The shared adapter reads the actual Shell bitmap dimensions before conversion and releases COM interfaces before balancing apartment initialization. Cache identities remain immutable and scans do not prune old files. Native Windows runtime validation remains unperformed on the macOS development host.

Application activation preserves the native launch entry. Windows application executables and original Shell Link `.lnk` files are opened through `ShellExecuteExW`, with COM initialized on the blocking launcher owner. The default Shell verb preserves shortcut arguments, working directory, window state, and Windows elevation behavior. Native errors, including cancelled elevation, return to the invoking action without a retry. macOS bundles continue through `/usr/bin/open`; the macOS adapter explicitly rejects Windows application descriptors. Direct program and shell-command descriptors retain their existing semantics. Shortcut icons are requested from the original Shell item so its configured resource and icon index are honored.

Persisted `target_path` is the native activation path: the original Windows shortcut or executable, or the macOS application bundle. Normalized `source_key` is a discovery comparison key, never a reconstructed launch path. Icon stamps use signed nanoseconds relative to the Unix epoch on both platforms, retaining valid pre-1970 timestamps. Icon metadata failures use the existing placeholder policy and report their concrete cause without excluding an otherwise valid application or making discovery partial.

Application discovery treats a missing scan root as an empty source on both supported platforms. Windows resolves known-folder paths without requiring those directories to exist; macOS retains its native application-root resolution. A complete scan removes entries whose source was deleted. Permission and other I/O failures remain partial scans and preserve unseen entries, so lack of access cannot be mistaken for deletion. Discovery never recreates a deleted directory.

Root Search refresh uses the same WebView `keydown` event on Windows and macOS. Only an unmodified F5 in a visible, focused launcher showing Root Search is accepted; no OS global shortcut is registered. The shell validates the launcher window and session and rejects refresh while an extension view or another operation is active. Rust refreshes extensions declaring `rootSearch` through their existing protocol workers, waits for explicit completion off the UI thread, and republishes the latest query through the session Channel. Accepted refreshes are bounded and never overwritten; failure remains explicit. Hiding the launcher does not cancel an already accepted refresh.

`engine/platform` owns the native mechanisms below. Runtime orchestration, configuration serialization and backups, and package validation and transactions remain in their existing engine modules. Callers use standard Rust paths, commands, child processes, and `io::Result`; native handles and platform selection never cross the adapter boundary.

| Contract                                       | Windows implementation                                                                                                                                                                                                              | macOS implementation                                                                                                                                                                                    |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Configure and attach an extension process tree | Create the child with `CREATE_NO_WINDOW` and `CREATE_SUSPENDED`, assign it to a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, then resume its initial thread. Both standard and asynchronous children follow this sequence. | Set a new process group before spawning. Both standard and asynchronous children retain that group for explicit termination.                                                                            |
| Terminate an extension process tree            | Terminate the Job Object and propagate native failures. Closing the owned Job handle retains the existing kill-on-close behavior.                                                                                                   | Send `SIGKILL` to the process group. Only an already absent group is accepted as terminated; other native failures propagate. Dropping the adapter does not introduce an additional termination policy. |
| Replace a prepared file                        | Use `MoveFileExW` with replacement and write-through flags.                                                                                                                                                                         | Use same-directory `rename`.                                                                                                                                                                            |
| Prepare a validated package executable         | No permission change is required.                                                                                                                                                                                                   | Set the executable's permissions to `0o755`.                                                                                                                                                            |
| Identify the package target                    | Select `x86_64-pc-windows-msvc` for x86_64.                                                                                                                                                                                         | Select `aarch64-apple-darwin` or `x86_64-apple-darwin` for the artifact architecture.                                                                                                                   |
| Locate an inventory-owned companion executable | Resolve the sibling executable with its `.exe` suffix.                                                                                                                                                                              | Resolve the sibling executable without a suffix.                                                                                                                                                        |
| Open a regular diagnostic file                 | Open the final component with `FILE_FLAG_OPEN_REPARSE_POINT`, then reject directories and reparse points from the opened handle's metadata.                                                                                         | Reject symlinks and non-files before opening; compare device and inode before and after opening to detect replacement.                                                                                  |
| Read and compare clipboard revisions           | Read `GetClipboardSequenceNumber`; compare the wrapping 32-bit sequence.                                                                                                                                                            | Read `NSPasteboard.changeCount`; compare the native-width wrapping counter.                                                                                                                             |

File replacement consumes a completed temporary file only on success. Configuration code remains responsible for writing and synchronizing temporary contents, preserving backups, and cleaning up its own failed transaction. The adapter does not add retries, recovery, timeouts, or a copy-and-delete fallback, and this contract does not add a cross-platform power-loss durability guarantee. Package code validates the entrypoint and decides when to apply executable permissions; the adapter does not choose or trust package paths.

Unsupported operating systems fail at the platform crate boundary. Unsupported artifact architectures retain an explicit unsupported package target, which package validation rejects. Process spawning, attachment, and cleanup must be tested together for both protocol and ACP extensions, including descendants created at startup. File tests cover replacement, creation, and preservation of the original destination on failure. Windows and macOS runtime validation remain distinct from cross-target compilation.

## Adding a future platform

Adding Linux or another target requires an explicit baseline decision, a complete adapter set for every required capability, target-specific packaging, CI coverage, physical validation, and release approval. The new adapter must preserve the existing shared contracts. No compatibility layer or migration path is required before the first release.

## Review gates

Every platform-facing change must identify:

1. the shared contract;
2. the macOS implementation;
3. the Windows implementation;
4. the unsupported-platform behavior;
5. the target-specific packaging and validation evidence.

An implementation is incomplete when a supported target silently uses another target's adapter, when a shared layer contains platform details, or when the two adapters expose different product semantics without an explicit product decision.
