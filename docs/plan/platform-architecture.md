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

`engine/platform` owns the adapter contracts and implementations. Its top-level module only selects the implementation for the build target and wires it to the shared contract. Tauri shell code owns only unavoidable application and window wiring. No extension, protocol, runtime, frontend, or domain module selects a platform implementation.

## Process and filesystem contracts

`engine/platform` owns the native mechanisms below. Runtime orchestration, configuration serialization and backups, and package validation and transactions remain in their existing engine modules. Callers use standard Rust paths, commands, child processes, and `io::Result`; native handles and platform selection never cross the adapter boundary.

| Contract | Windows implementation | macOS implementation |
| --- | --- | --- |
| Configure and attach an extension process tree | Create the child with `CREATE_NO_WINDOW` and `CREATE_SUSPENDED`, assign it to a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, then resume its initial thread. Both standard and asynchronous children follow this sequence. | Set a new process group before spawning. Both standard and asynchronous children retain that group for explicit termination. |
| Terminate an extension process tree | Terminate the Job Object and propagate native failures. Closing the owned Job handle retains the existing kill-on-close behavior. | Send `SIGKILL` to the process group. Only an already absent group is accepted as terminated; other native failures propagate. Dropping the adapter does not introduce an additional termination policy. |
| Replace a prepared file | Use `MoveFileExW` with replacement and write-through flags. | Use same-directory `rename`. |
| Prepare a validated package executable | No permission change is required. | Set the executable's permissions to `0o755`. |
| Identify the package target | Select `x86_64-pc-windows-msvc` for x86_64. | Select `aarch64-apple-darwin` or `x86_64-apple-darwin` for the artifact architecture. |
| Locate an inventory-owned companion executable | Resolve the sibling executable with its `.exe` suffix. | Resolve the sibling executable without a suffix. |
| Open a regular diagnostic file | Open the final component with `FILE_FLAG_OPEN_REPARSE_POINT`, then reject directories and reparse points from the opened handle's metadata. | Reject symlinks and non-files before opening; compare device and inode before and after opening to detect replacement. |

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
