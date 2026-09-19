# Nanika Project Instructions

## Workflow and layout

- Treat the current pre-release design as the only baseline. Rewrite unpublished schemas and formats directly; leave no stale design, compatibility paths, migrations, or known technical debt when completing a stage.
- Use `just dev` to start the complete application and `just check` for repository validation. Toolchain pins and delegated commands live in the repository manifests and `justfile`. Keep the current Active LTS Node.js line pinned for development and CI; development tools must not enter production artifacts.
- Organize by responsibility: `apps` contains deployable processes, `engine` contains reusable UI-independent behavior, and `tooling` contains repository build, quality, and release support. Keep tests and benchmarks with their owning module.
- Keep Svelte presentation and browser assets in `apps/desktop/frontend`; keep Tauri configuration, capabilities, resources, and privileged shell wiring in `apps/desktop/shell`. The CLI belongs in `apps/cli`; built-in executables and test fixtures belong in `apps/extensions`.
- `engine/foundation` (`nanika-foundation`) owns project identity, extension IDs, and diagnostic primitives. `engine/platform` owns shared native mechanisms behind adapters; runtime, configuration, and extension management retain orchestration and transaction policy.
- Keep temporary repository outputs and benchmark data under `target`. Do not add top-level `crates`, `extensions`, `src-tauri`, `web`, `rust`, `scripts`, `packaging`, or `dist`; frontend build output stays in `apps/desktop/frontend/dist`.

Detailed baseline: [technical stack](docs/plan/tech-stack.md). Distinguish implemented behavior from planned work in [tasks](docs/plan/tasks.md) and [release](docs/plan/release.md).

## Behavior and platform boundaries

- Preserve literal failure semantics. Do not add automatic timeouts, retries, restarts, truncation, retention, deletion, recovery, or fallback that changes behavior unless its exact policy is documented and approved. Pending work stays pending; failures retain their concrete cause.
- Apply backpressure to accepted work instead of dropping it. Coalesce only idempotent wake signals or superseded queries whose authoritative latest value is stored separately.
- Support and release only Windows 10+ and macOS 13+. Linux and other platforms must fail explicitly at the adapter boundary, with no implicit fallback.
- Before platform-facing changes, document the shared contract and both Windows and macOS implementations. Keep native APIs, handles, OS-specific paths, and platform conditionals inside adapters, outside shared engine, protocol, frontend, and extension code. Conditionals must name a supported platform or an explicit unsupported branch. Exceptions require a review note naming platforms, semantics, and validation evidence.
- Adapters may vary mechanisms, not product semantics. Process containment, file replacement, executable permissions, target selection, and diagnostic file opening belong in `engine/platform`; callers retain lifecycle, validation, and transaction decisions. Validate changes on both supported platforms.

Contract details and review gates: [platform architecture](docs/plan/platform-architecture.md).

## Core, extensions, and IPC

- Rust owns search infrastructure, extension supervision, storage, configuration, diagnostics, and platform services. Keep the core independent of Tauri types. Tauri commands, channels, windows, lifecycle, capabilities, and custom protocols belong in the desktop shell.
- Extensions are the only domain capability unit. The bare host provides infrastructure, orchestration, and shared surfaces only; application discovery, calculation, clipboard history, commands, scripts, and agents belong in extensions.
- Built-in and external extensions must share manifests, protocol, process boundaries, permissions, host services, configuration contributions, declarative views, lifecycle, failure policy, and diagnostics. Built-in identity comes from host-owned distribution inventory verified by the signed release, never from extension-controlled fields, and grants no shortcut.
- One shared Svelte frontend renders launcher, Settings, diagnostic, and extension surfaces. The shell owns unavoidable OS surfaces and recovery before the WebView is available. Extensions provide bounded data, declarative nodes, and typed actions only, with no injected HTML, CSS, JavaScript, Svelte components, arbitrary drawing, DOM, WebView, or Tauri access. Never render application or extension data with `{@html}`.
- Use bounded serializable Rust/frontend contracts, explicit Tauri command permissions, Rust request and scope validation, and channels bound to the authorized window session. Expose no raw filesystem or process access.
- Submit search through `invoke`; deliver search state through one long-lived Rust-to-frontend Channel per WebView session. Query replies acknowledge submission without updating result lists. Preserve Isolation's required internal transport, bound delivery, and distinguish queued sends from acknowledged frontend receipt.

## Frontend and responsiveness

- Tauri 2 is the only desktop baseline. Use the latest mutually compatible stable Svelte 5, TypeScript, Vite, pnpm, and Tauri APIs. Prefer suitable Tauri core APIs and official plugins; avoid UI compatibility layers, dual UI paths, experimental APIs, unstable Cargo features, broad permissions, and unnecessary plugins.
- Use Svelte 5 runes and current component/event syntax. Reserve `$effect` for synchronization with external systems, never ordinary derived state. Use plain CSS, semantic tokens, and shared components with a coherent visual language and clear hierarchy.
- Use Nanika domain terms and responsibilities for code and identifiers. Comments must describe behavior, ownership, and constraints without mentioning third-party software or reference applications. Do not name implementations after other applications or describe them as copies.
- Prefer semantic HTML, native text editing, browser focus, and established ARIA patterns. Treat native window effects as measured progressive enhancement. Use Svelte transitions and CSS motion with explicit timing, easing, interruption, and reduced-motion behavior.
- Do not add SvelteKit, a router, global state framework, component library, utility CSS, CSS-in-JS, simulated DOM, or animation library without a demonstrated requirement.
- Keep blocking work off the Tauri event-loop and WebView main thread. Hidden UI must not poll or run animation frame loops. Measure latency, frame pacing, and resource use; maintain smooth 60 Hz and 120 Hz behavior where available.

Presentation and measurement details: [UI](docs/plan/ui.md) and [performance](docs/plan/performance.md).

## Validation

- Automated tests cover Rust behavior, failure isolation, protocol and storage contracts, and concurrency. Preserve zero-extension host tests, built-in/external equivalence tests, extension failure tests, and inventory checks rejecting extension-supplied frontend code.
- Run repository checks, including Rust formatting, Clippy, tests, documentation, frontend build, `svelte-check`, ESLint through `eslint-config-zoro` with Svelte support, and dprint. Treat warnings as failures in CI; validation dependencies are development-only.
- Validate UI with computer-use in the actual Tauri application on Windows and macOS, including high-DPI displays. Exercise semantic roles, accessible names, keyboard and pointer input, focus, and rendered state; do not test private component implementation.
- Measure representative core workloads and actual Nanika startup, extension execution, search, IPC, rendering, scrolling, and resource use. Passing assertions or cross-target compilation do not establish platform runtime, UI, or end-to-end performance correctness. Report unvalidated platforms explicitly.
