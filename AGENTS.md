# Nanika

## Workflow

- Make breaking pre-release changes directly; remove superseded code. No compatibility paths, migrations or automatic data resets.
- Use direct file edits, not apply_patch. Keep changes unstaged unless asked.
- Use just dev and just check. Keep one root package and bun.lock, the pinned Bun version and public registries. Keep credentials and private infrastructure out of code, logs and artifacts.
- Preserve apps / engine / tooling ownership. Temporary output belongs in target; frontend output in apps/desktop/frontend/dist.
- Bun is tooling only. Installed releases must need neither Bun nor Node. Derive sidecar build/staging from Tauri bundle.externalBin; inspect packaged files after toolchain or packaging changes.

## Architecture

- Support Windows 10+ and macOS 13+ only. Keep native mechanisms in their owning adapters, document the shared contract and both implementations, and reject other platforms explicitly.
- Rust owns search, supervision, storage, configuration and platform services. Keep engine independent of Tauri; commands, channels, windows and capabilities belong in the desktop shell. Callers own lifecycle and transaction policy.
- Extensions provide all domain capabilities. Built-in and external extensions share process, permission, configuration, view, lifecycle and failure contracts. Only host inventory establishes built-in identity; provenance grants no runtime shortcut.
- One Svelte frontend renders bounded declarative extension data. No extension-supplied frontend code or DOM/WebView access; never render application or extension data with {@html}.
- Route frontend Tauri access through the typed bridge. Preserve Isolation, explicit permissions, Rust validation and session-bound channels. Search invoke replies acknowledge submission; authoritative state arrives through the Channel. Queued delivery is not receipt.
- Preserve accepted work, concrete failures and bounded backpressure. Coalesce only idempotent wakes or superseded queries/selections where the latest value remains authoritative; actions are barriers. No new automatic timeout, retry, restart, truncation, retention, deletion, recovery or fallback policy without approval.
- Extension live enable/disable is deferred until the foundation is committed and implementation is separately authorized. Do not turn its proposal into runtime code prematurely.

## UI

- Use Svelte 5 runes, Bits UI headless primitives, shared controls and plain CSS semantic tokens. Reserve $effect for external synchronization. Follow docs/design-system.md; use Fluent 2 as the design system, adapting for native conventions or concrete product needs. Do not adopt its component library/theme or raise OS/WebView requirements.
- Keep native editing, selection and keyboard semantics. No focus rings, extra Actions button or unrequested shortcuts. Tauri owns launcher hiding; DOM activity owns visual work only. Settings has no custom window transition.
- Motion must handle interruption and live reduced-motion preferences. Hidden UI must not poll or animate; keep blocking work off UI/event-loop threads. Add dependencies only for demonstrated requirements.

## Validation and docs

- Run repository checks appropriate to the change. Preserve zero-extension, extension-equivalence, failure, protocol/storage and concurrency coverage. Keep frontend/tooling tests and types separate; validation dependencies stay development-only and CI warnings fail.
- Validate affected Tauri flows on both platforms, including focus, input, DPI and rendered state. Measure performance changes under comparable workloads, including 60/120 Hz and hidden idle where relevant. Report unvalidated platforms; browser fixtures and cross-compilation do not prove native behavior.
- Code and manifests define implementation. Update the existing documents, remove completed/history-only content, and keep optional candidates distinct from commitments. Original artwork belongs in docs/assets/icons; runtime and packaging exports stay with their consumers.

References: [architecture](docs/platform-architecture.md), [extensions](docs/extension-lifecycle.md), [design system](docs/design-system.md), [unfinished candidates](docs/tasks.md).
