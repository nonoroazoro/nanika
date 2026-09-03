# Nanika Project Instructions

## Development

- Before the first release, treat the current design as the only baseline. Rewrite unpublished schemas and formats instead of preserving compatibility or migrations.
- Do not complete a stage with stale design, dead compatibility paths, or known technical debt.

## Performance

- Performance is a first-class requirement: keep the UI responsive and measure latency, frame pacing, and resource use.
- Keep blocking work off the Tauri event-loop and WebView main thread. Hidden UI must not poll or run an animation frame loop.

## Cross-platform

- Shared host, UI, diagnostics, protocol, and extension behavior must remain platform-neutral.
- Isolate unavoidable OS-specific behavior behind platform adapters and maintain implementations for every supported OS. Never introduce a single-OS solution into shared code.
- Validate cross-platform changes on Windows and macOS. Linux-specific behavior must not become an implicit fallback or enter shared paths unless Linux is added to the supported baseline.

## Architecture

- Rust remains the application core for search, extension supervision, storage, configuration, diagnostics, and platform services.
- Tauri is the only desktop shell and UI baseline. The frontend uses the latest mutually compatible stable Svelte 5, TypeScript, Vite, and pnpm releases. Use plain CSS for the design system.
- Organize repository source by product responsibility, not implementation language. Deployable processes belong under `apps`, reusable UI-independent application behavior belongs under `engine`, and repository-only quality and release support belongs under `tooling`.
- Keep the desktop presentation layer in `apps/desktop/frontend` and the privileged desktop boundary in `apps/desktop/shell`. Do not mix Svelte source, frontend tests, or browser assets with Rust shell source, Tauri configuration, capabilities, native resources, or platform lifecycle code.
- Do not add top-level `crates`, `extensions`, `src-tauri`, `web`, `rust`, `scripts`, `packaging`, or generated `dist` directories. Built-in extension executables and extension test fixtures belong under `apps/extensions`; the CLI belongs under `apps/cli`.
- Pin the current Active LTS Node.js line for frontend development and CI. Node.js, frontend build tools, static-analysis tools, and test tools must not enter production artifacts.
- Tauri is the current design, not a compatibility target. Do not create UI compatibility layers or dual UI paths.
- Use Svelte 5 runes and current component syntax. Do not introduce Svelte legacy reactivity or event syntax. Use `$effect` only to synchronize with an external system, never to derive ordinary state.
- Prefer the newest mutually compatible stable Tauri 2 core APIs and official plugins when they satisfy the product contract. Track current Tauri architecture and security guidance instead of carrying custom desktop glue by default.
- Advanced means stable, high-level, secure, and measured. Do not enable experimental APIs, unstable Cargo features, broad plugin permissions, or unnecessary plugins merely because they are newer.
- Keep the Rust core independent of Tauri types. Tauri commands, channels, lifecycle events, windows, capabilities, and custom protocols belong to the desktop shell boundary.
- Define bounded serializable contracts between Rust and the frontend. Grant application commands through explicit Tauri permissions, validate every request and scope in Rust, and bind channels to the authorized window session. Expose no raw filesystem or process access.
- Render extension and diagnostic content as text or bounded declarative nodes. Never use Svelte `{@html}` for application or extension data.

## Testing

- Testing is a first-class architecture requirement. Use Vitest for pure TypeScript tests and Vitest Browser Mode with its Playwright provider and official Svelte renderer for component and interaction tests in real Chromium and WebKit engines.
- Run `svelte-check`, ESLint with Svelte support, and Prettier with Svelte support. Treat warnings as failures in CI. All test and validation packages are development-only and must not enter production assets.
- Mock Tauri only at the typed frontend bridge. Browser tests do not replace release-equivalent Tauri black-box and physical acceptance testing on Windows and macOS.
- Test user-visible behavior through semantic roles, accessible names, keyboard and pointer input, focus, rendered state, and stable contracts. Do not test private component implementation.

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
