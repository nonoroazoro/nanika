# Technical Stack: Open Work

The code, manifests, and repository tooling define implemented behavior. This file records only unfinished architecture decisions. See [tasks](tasks.md) for actionable work and [release](release.md) for acceptance gates.

## Extension boundary

- Derive the built-in extension inventory from reviewed manifests and verify its identities, executable pairing, and contents in signed release artifacts. Extension-controlled fields must not establish built-in trust.
- Audit shell, frontend, engine, and storage for capability-specific branches. Domain behavior belongs in extensions; shared surfaces consume typed protocol data.
- Complete zero-extension, independent-failure, built-in/external equivalence, and request-correlation tests. Architecture checks should reject domain implementations outside `apps/extensions`, extension-owned frontend code, Tauri dependencies in `engine`, and overbroad shell permissions.
- Deliver invocation completion, streamed output, view updates, configuration application, diagnostics, and runtime state through session-bound desktop channels. Preserve request identity, ordering, backpressure, and explicit failure. Query acknowledgements must not mutate result lists.

## Desktop integration

- Complete startup enablement and stale-instance handling on both supported platforms. Correlate native hotkey delivery, window visibility, frontend commit, input focus, and interactive readiness without treating visibility as readiness.
- Validate the packaged Isolation policy and Channel behavior on WKWebView and WebView2, including payloads on both sides of Tauri's delivery threshold and frontend receipt acknowledgements.
- Keep platform-specific APIs inside `engine/platform` adapters or unavoidable Tauri shell wiring. Before adding a new platform-facing behavior, define one shared semantic contract, both supported implementations, explicit unsupported behavior, and runtime validation.

## Shared search and configuration

- `engine/text-search` is reusable by other features, but Settings and Clipboard do not yet use its matcher. Integrate it only when their search contracts and workloads are defined; avoid an automatic index or per-keystroke rebuild.
- Add typed, bundled shell message catalogs selected by the operating-system locale with deterministic English fallback. Locale changes must not alter identity, focus, or navigation.
- Define a separate platform-neutral paste-to-foreground host service before exposing paste behavior. It needs explicit Windows and macOS adapters, permission checks, and failure semantics.

## Deferred decisions

- Define migration, backup, and rollback rules when the first released persistent format needs a change. Pre-release schemas have no compatibility obligation.
- Define machine overrides, secrets, child-output capture, and launched-process cancellation only when a concrete capability requires them.
- Do not add a new platform, marketplace, background updater, or enforceable native-extension sandbox without a separate product and security decision.
