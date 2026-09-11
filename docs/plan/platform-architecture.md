# Platform Architecture Baseline

## Supported targets

Nanika currently supports and releases only:

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

The frontend is one shared Svelte application for both release targets. Tauri exposes the same typed commands and channel contracts on macOS and Windows. Target-specific window, tray, shortcut, process, clipboard, and startup behavior stays in the shell wiring or platform adapter and is excluded from shared presentation state.

Platform adapters own native APIs, handles, event sources, process containment, window placement, startup registration, single-instance activation, clipboard monitoring, and other target-specific mechanisms. Adapters expose the same contract, failure boundary, cancellation behavior, lifecycle, and diagnostics shape on macOS and Windows.

`engine/platform` owns the adapter contracts and implementations. Its top-level module only selects the implementation for the build target and wires it to the shared contract. Tauri shell code owns only unavoidable application and window wiring. No extension, protocol, runtime, frontend, or domain module selects a platform implementation.

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
