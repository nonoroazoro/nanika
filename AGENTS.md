# Nanika Project Instructions

## Development

- Before the first release, treat the current design as the only baseline. Rewrite unpublished schemas and formats instead of preserving compatibility or migrations.
- Do not complete a stage with stale design, dead compatibility paths, or known technical debt.
- Keep failure semantics literal. Do not add automatic timeouts, retries, restarts, truncation, retention, deletion, recovery, or fallback that changes product behavior unless the exact policy is documented and approved. Pending work stays pending, failures remain explicit, and diagnostics retain the concrete cause.
- Apply backpressure to accepted work instead of dropping it. Coalescing is allowed only for idempotent wake signals or superseded queries whose latest authoritative value is stored separately.

## Performance

- Performance is a first-class requirement: keep the UI responsive and measure latency, frame pacing, and resource use.
- Keep blocking work off the Tauri event-loop and WebView main thread. Hidden UI must not poll or run an animation frame loop.

## Cross-platform

- Shared host, UI, diagnostics, protocol, and extension behavior must remain platform-neutral.
- Isolate unavoidable OS-specific behavior behind platform adapters and maintain implementations for every supported OS. Never introduce a single-OS solution into shared code.
- The supported platform baseline is macOS 13+ and Windows 10+. Release artifacts are produced only for these two platforms. Linux and every other platform are explicitly unsupported and must not receive an implicit fallback implementation.
- Validate cross-platform changes on both macOS and Windows. Platform conditionals must name one of the supported platforms or an explicit unsupported-platform branch.
- Before implementing any platform-facing feature, document its Windows and macOS implementations and its shared, platform-neutral contract. Platform conditionals, native handles, OS paths, and native API types are forbidden in shared engine, protocol, frontend, and extension code; an unsupported platform must fail explicitly behind the platform adapter rather than silently using another OS as a fallback. Any exception requires a review note naming the affected platforms, semantics, and validation evidence.

## Architecture

- Rust remains the application core for search, extension supervision, storage, configuration, diagnostics, and platform services.
- Tauri is the only desktop shell and UI baseline. The frontend uses the latest mutually compatible stable Svelte 5, TypeScript, Vite, and pnpm releases. Use plain CSS for the design system.
- Extensions are Nanika's only first-class domain capability unit. The bare host provides infrastructure, orchestration, and shared surfaces only; it must not implement application search, commands, scripts, calculation, clipboard history, agents, or any future domain capability.
- Built-in and external extensions use the same manifest schema, protocol, process boundary, permissions, host services, settings contribution, declarative view contract, lifecycle, failure policy, and diagnostics. Built-in identity comes only from host-owned distribution inventory, never from an extension-controlled manifest field. Production verifies that inventory as part of the signed release. Built-in identity grants no architectural or runtime shortcut.
- The shared Svelte frontend renders every launcher, Settings, diagnostic, and extension view surface. The Tauri shell owns only unavoidable operating-system surfaces and recovery before the WebView is available. Extensions provide data, bounded declarative views, and typed actions only; they never provide or inject HTML, CSS, JavaScript, Svelte components, DOM access, WebView access, or Tauri access.
- Organize repository source by product responsibility, not implementation language. Deployable processes belong under `apps`, reusable UI-independent application behavior belongs under `engine`, and repository-only quality and release support belongs under `tooling`.
- Keep the desktop presentation layer in `apps/desktop/frontend` and the privileged desktop boundary in `apps/desktop/shell`. Do not mix Svelte source or browser assets with Rust shell source, Tauri configuration, capabilities, native resources, or platform lifecycle code.
- Do not add top-level `crates`, `extensions`, `src-tauri`, `web`, `rust`, `scripts`, `packaging`, or generated `dist` directories. Built-in extension executables and extension test fixtures belong under `apps/extensions`; the CLI belongs under `apps/cli`.
- Pin the current Active LTS Node.js line for frontend development and CI. Node.js, frontend build tools, static-analysis tools, and test tools must not enter production artifacts.
- Tauri is the current design, not a compatibility target. Do not create UI compatibility layers or dual UI paths.
- Use Svelte 5 runes and current component syntax. Do not introduce Svelte legacy reactivity or event syntax. Use `$effect` only to synchronize with an external system, never to derive ordinary state.
- Prefer the newest mutually compatible stable Tauri 2 core APIs and official plugins when they satisfy the product contract. Track current Tauri architecture and security guidance instead of carrying custom desktop glue by default.
- Advanced means stable, high-level, secure, and measured. Do not enable experimental APIs, unstable Cargo features, broad plugin permissions, or unnecessary plugins merely because they are newer.
- Keep the Rust core independent of Tauri types. Tauri commands, channels, lifecycle events, windows, capabilities, and custom protocols belong to the desktop shell boundary.
- Define bounded serializable contracts between Rust and the frontend. Grant application commands through explicit Tauri permissions, validate every request and scope in Rust, and bind channels to the authorized window session. Expose no raw filesystem or process access.
- Submit search requests through Tauri `invoke` and deliver search state through one long-lived Rust-to-frontend Channel per WebView session. Query RPC replies must not update result lists. Preserve Tauri's required internal transport through Isolation, bound delivery, and distinguish queued sends from acknowledged frontend receipt.
- Render extension and diagnostic content as text or bounded declarative nodes. Never use Svelte `{@html}` for application or extension data.

## Testing

- Automated tests cover Rust. Validate UI behavior with computer-use in the actual Tauri application, including its Rust core, extensions, IPC, and platform WebView.
- Rust core tests must cover behavior, failure isolation, protocol and storage contracts, and concurrency correctness. Measure core latency and resource use with representative workloads; test counts and passing assertions do not establish performance or UI correctness.
- Observe end-to-end performance in the actual Nanika process, including startup, extensions, Rust search, Tauri IPC, UI rendering, scrolling, and resource use. Use computer-use for other UI behavior and visual acceptance.
- Run `svelte-check`, ESLint through `eslint-config-zoro` with Svelte support, and dprint. Treat warnings as failures in CI. All test and validation packages are development-only and must not enter production assets.
- Validate the actual application on Windows and macOS; isolated Rust tests do not prove UI behavior or end-to-end performance.
- Test user-visible behavior through semantic roles, accessible names, keyboard and pointer input, focus, rendered state, and stable contracts. Do not test private component implementation.
- Prove the extension-first model with zero-extension host tests, built-in and external equivalence tests, extension failure-isolation tests, protocol contract tests, and release inventory checks that reject extension-supplied frontend code.

## UI

- The UI must be elegant and coherent, with an explicit visual language and clear hierarchy.
- Validate the experience on Windows and macOS, including high-DPI displays.
- Prefer semantic HTML, browser focus behavior, native text editing, and established ARIA interaction patterns over custom input or list mechanics.
- Prefer current stable Web platform and Tauri presentation capabilities over legacy compatibility code. Treat native window effects as measured progressive enhancement, never as a requirement for hierarchy or legibility.
- Build a shared frontend design system from semantic tokens and reusable components. Extensions provide declarative content and actions, never HTML, CSS, scripts, or arbitrary drawing access.
- Use Svelte's built-in reactivity and transition facilities plus CSS motion primitives. Do not add SvelteKit, a router, global state framework, component library, utility CSS framework, CSS-in-JS runtime, simulated DOM, or animation library without a demonstrated requirement.

## Animation

- Motion must be fluid, purposeful, and state-driven rather than decorative.
- Define timing, easing, interruption, and reduced-motion behavior; maintain smooth frame pacing at 60 Hz and 120 Hz where available.
