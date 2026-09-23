# Nanika Project Instructions

## Workflow

- Treat the pre-release design as the only baseline. Make breaking changes directly; leave no compatibility paths, migrations, stale designs, or known technical debt.
- Keep one root JavaScript package and `bun.lock`. Use the pinned Bun version and public registries; inspect lockfile download sources. Exclude employer identifiers, internal infrastructure references, and credentials from files, logs, and artifacts.
- Bun is tooling only. Never ship its runtime, compile tooling into shipped executables, or invoke it from the installed app. Releases must run without Bun or Node; inspect packaged files and native dependencies after toolchain or packaging changes.
- Prefer Rust and TypeScript tooling with suitable Bun APIs. Keep browser and tooling types separate. Use `just dev` and `just check` as repository entry points.
- Preserve the `apps` / `engine` / `tooling` responsibility split. Do not add parallel top-level source trees. Temporary outputs belong under `target`; frontend output belongs in `apps/desktop/frontend/dist`.
- Derive extension build and staging inputs from Tauri `bundle.externalBin`; do not maintain another executable list.

## Platform and failure semantics

- Support only Windows 10+ and macOS 13+; reject other platforms explicitly. Before platform changes, document the shared contract and both implementations. Keep native APIs, handles, paths, and conditionals inside adapters; exceptions require a review note with validation evidence.
- `engine/foundation` owns identity, extension IDs, and diagnostic primitives. `engine/platform` owns native mechanisms, including process containment, file replacement, executable permissions, target selection, and diagnostic opening. Callers retain lifecycle and transaction policy; adapters must preserve product semantics.
- Preserve pending work and concrete failure causes. No automatic timeouts, retries, restarts, truncation, retention, deletion, recovery, or fallback without an approved policy. Apply backpressure; coalesce only idempotent wakes or superseded queries whose latest value remains authoritative.

## Core, extensions, and IPC

- Rust owns search, supervision, storage, configuration, diagnostics, and platform services. Keep engine crates independent of Tauri; Tauri commands, channels, windows, lifecycle, capabilities, and protocols belong in the desktop shell.
- Extensions are the only domain capability unit. Built-in and external extensions share all runtime contracts, including process boundaries, permissions, configuration, views, lifecycle, failure policy, and diagnostics. Built-in identity comes only from host-owned inventory verified by the signed release and grants no shortcut.
- One Svelte frontend renders all application and extension surfaces. Extensions supply bounded data, declarative nodes, and typed actions, never injected HTML/CSS/JavaScript, components, drawing, DOM, WebView, or Tauri access. Never render application or extension data with `{@html}`. The shell owns unavoidable OS surfaces and pre-WebView recovery.
- Use bounded serializable contracts, explicit command permissions, Rust request/scope validation, and session-bound channels. Frontend Tauri access goes through the typed bridge; expose no raw filesystem or process access.
- Submit search through `invoke`; deliver state through one long-lived Channel per WebView session. Replies acknowledge submission without changing result lists. Preserve Isolation transport and bounded delivery; distinguish queued sends from acknowledged receipt.

## Frontend

- Use Svelte 5 runes and current event syntax. Reserve `$effect` for external synchronization. Use plain CSS, semantic tokens, and shared components.
- Prefer native HTML editing/focus and established ARIA patterns. Motion must define timing, interruption, and reduced-motion behavior. Native window effects require measurement.
- Add routers, state frameworks, UI/style libraries, simulated DOM, or animation libraries only for demonstrated requirements. Avoid experimental APIs, unstable Cargo features, broad permissions, and reference-product naming or comments.
- Keep blocking work off event-loop and WebView threads. Hidden UI must not poll or animate. Measure latency, frame pacing, and resource use for 60 Hz and 120 Hz displays.

## Validation

- Preserve zero-extension host, built-in/external equivalence, extension failure, protocol/storage, and concurrency tests. Inventory checks must reject extension-supplied frontend code.
- Keep Vitest projects for frontend and tooling, reusing frontend Vite configuration. Use `eslint-config-zoro` with Svelte support and dprint. Treat CI warnings as failures; validation dependencies remain development-only.
- Validate actual Tauri UI on both platforms, including high DPI, accessibility, keyboard/pointer input, focus, and rendered state. Measure actual startup, extension execution, search, IPC, rendering, scrolling, and resource use. Unit tests and cross-compilation do not prove runtime or UI correctness; report unvalidated platforms explicitly.

Code and manifests define implemented behavior. Open work: [stack](docs/plan/tech-stack.md), [tasks](docs/plan/tasks.md), [platform](docs/plan/platform-architecture.md), [UI](docs/plan/ui.md), [performance](docs/plan/performance.md), [release](docs/plan/release.md).
