# Nanika Tasks

Status: Tauri is the only pre-1.0 desktop baseline. Every unchecked item is a TODO.

## Product invariants

- Extensions are the only first-class domain capability unit.
- The bare host owns infrastructure, orchestration, shared control-plane surfaces, and presentation contracts. It contributes no domain candidate or action.
- Built-in and external extensions use the same process, protocol, permission, view, action, failure, and diagnostics paths.
- Built-in status is host-owned distribution metadata. It grants no runtime privilege.
- Rust owns search, storage, configuration, diagnostics, extension supervision, host services, and platform integration.
- Tauri owns the desktop shell. Svelte 5, TypeScript, Vite, and plain CSS own presentation and local interaction.
- Extensions provide bounded data and typed actions. They never provide frontend code, HTML, CSS, scripts, components, remote UI, DOM access, or Tauri access.
- There is no compatibility layer, parallel UI, or migration path for the unpublished renderer and pre-release schemas.

## Completed foundation

- [x] Reorganize the repository by product responsibility under `apps`, `engine`, and `tooling`.
- [x] Move the CLI to `apps/cli`, shipping extensions to `apps/extensions/built-in`, protocol fixtures to `apps/extensions/fixtures`, shared behavior to `engine`, and repository support to `tooling`.
- [x] Remove the top-level `crates`, `extensions`, `scripts`, `packaging`, and root `dist` layouts.
- [x] Remove every superseded desktop presentation path, native integration duplicate, test, benchmark, asset, and dependency.
- [x] Make the current database schemas the only pre-release development baseline.
- [x] Create `apps/desktop/frontend` and `apps/desktop/shell` as separate presentation and privileged desktop boundaries.
- [x] Pin the current Active LTS Node.js line and exact mutually compatible stable frontend, Tauri, and test tool versions.
- [x] Configure Svelte 5, TypeScript, Vite, pnpm, Vitest, Vitest Browser Mode, Playwright, `svelte-check`, ESLint, and Prettier without SvelteKit or a pnpm workspace.
- [x] Create the Tauri 2 shell with a hidden launcher window, explicit capabilities, command pruning, restrictive CSP, Isolation Pattern, disabled asset protocol, tray ownership, global shortcut ownership, single-instance activation, and active-monitor placement.
- [x] Move search, extension supervision, storage, and permission-checked host services behind the UI-independent `engine/runtime` service.
- [x] Add reviewed distribution inventory under `apps/extensions` instead of a compiled built-in registry.
- [x] Connect Root Search through bounded Rust DTOs, explicit Tauri commands, a session-bound Tauri channel, and one typed frontend bridge.
- [x] Add a validated `nanika-icon` custom protocol backed by a bounded off-event-loop reader and immutable extension-scoped cache identities.
- [x] Add the initial semantic combobox/listbox implementation with native text editing, clamped keyboard selection, pointer activation, and query selection on WebView mount.
- [x] Replace the repository quality entry points with `tooling/quality` checks for Rust, frontend formatting, linting, type analysis, browser tests, production builds, and initial architecture boundaries.

## Extension-first completion

- [ ] Add ordinary manifests for every bundled extension and derive the development inventory from validated manifest data.
- [ ] Stage and declare built-in executables through Tauri `bundle.externalBin` with target-triple filenames. Verify the same inventory in signed release artifacts.
- [ ] Reject external packages that attempt to assert built-in identity or replace a reserved built-in extension.
- [ ] Complete zero-extension startup, partial failure, restart, disablement, and independent-host contract tests.
- [ ] Audit shell, frontend, engine, and storage boundaries for capability-specific branches. Move capability behavior into extensions or replace presentation-only branching with protocol metadata.
- [ ] Deliver invocation completion, bounded streaming output, extension view updates, settings updates, diagnostics, and runtime state through session-bound Tauri channels.
- [ ] Render extension List, Split, Detail, filter, pagination, nested navigation, Back, and typed actions through shared Svelte components.
- [ ] Route every extension view action through Rust authorization and the versioned protocol to its owning extension.
- [ ] Render Settings from the bounded shared settings contract and restore the Settings tray action only when that surface works.
- [ ] Add architecture checks that reject domain implementations outside `apps/extensions`, extension-specific frontend components, extension-owned Web assets, Tauri dependencies in `engine`, and removed top-level layouts.

## Desktop shell

- [ ] Make runtime initialization failures visible through a bounded native or frontend diagnostic instead of logging only.
- [ ] Consume invocation navigation effects without polling and keep the launcher open only for actions that open shared views.
- [ ] Complete startup enablement, settings-window lifecycle, shutdown coordination, and stale instance recovery through Tauri and platform adapters.
- [ ] Add frontend readiness, window visibility, focus, and interactive activation milestones while retaining passive native hotkey delivery timing.
- [ ] Validate the Isolation Pattern payload policy against channels, every command shape, and measured IPC overhead.
- [ ] Validate transparent-window startup, borders, shadows, focus, and active-monitor placement on physical Windows and macOS systems.
- [ ] Evaluate stable native window effects only as measured progressive enhancement with a complete semantic CSS fallback.
- [ ] Keep native tray actions limited to Open Nanika, Settings, and Quit. Application refresh remains an Application Extension action.

## Frontend architecture and design system

- [ ] Implement the design principles and interaction rules in `ui.md` without redefining behavior inside feature components.
- [ ] Expand semantic tokens for typography, color, spacing, size, radius, elevation, motion, and interaction states using plain CSS.
- [ ] Implement shared Svelte primitives for SearchInput, ResultList, ResultRow, SectionHeader, ActionBar, KeyHint, DetailPanel, EmptyState, LoadingState, and DiagnosticState.
- [ ] Keep Tauri imports inside the typed bridge. Components consume application services and typed snapshots only.
- [ ] Use Svelte 5 runes and current event syntax. Use `$derived` for derived state and `$effect` only for external synchronization.
- [ ] Add `<svelte:boundary>` around the application root and extension route content. Handle asynchronous and command failures explicitly.
- [ ] Prohibit `{@html}` for application and extension data.
- [ ] Add bundled typed message catalogs selected from the operating-system locale with deterministic English fallback.
- [ ] Render localized application titles while preserving original names as search aliases.
- [ ] Use browser scrolling as the source of truth and reveal the active option only when it crosses the scrollport boundary.
- [ ] Keep the initial 100-result list non-virtualized unless measurement demonstrates a need.
- [ ] Define coherent loading, empty, degraded, recoverable error, and unavailable states before declaring a surface complete.

## Testing

- [x] Configure separate Vitest Node and Browser Mode projects with the Playwright provider and official Svelte renderer.
- [x] Keep frontend tests and static-analysis packages development-only.
- [ ] Contract-test matching Rust and TypeScript request, response, snapshot, channel, lifecycle, and error shapes.
- [ ] Test Root Search typing, IME boundaries, Enter, Up and Down clamping, boundary-only scrolling, pointer activation, stable snapshots, reopen selection, icon completion, and pressed-state release.
- [ ] Test shared components through semantic roles, accessible names, keyboard and pointer input, focus, rendered state, themes, reduced motion, and failures.
- [ ] Test extension presentation from validated declarative fixtures through the typed bridge and back to the owning extension identity.
- [ ] Add bounded visual regression for stable shared primitives in Chromium and WebKit.
- [ ] Add release-equivalent Tauri black-box tests for WebView2 and WKWebView. Browser Mode does not replace them.
- [ ] Validate all supported behavior on physical Windows and macOS systems, including high DPI, mixed DPI, CJK IME, accessibility scaling, 60 Hz, and 120 Hz.

## Performance and release

- [ ] Add Tauri summon, first-paint, focus, input-to-results, scrolling, memory, and hidden-idle measurements.
- [ ] Record JavaScript and CSS bytes, chunk count, source-map absence, parse time, and evaluation time for production frontend builds.
- [ ] Measure channel, Isolation Pattern, icon protocol, extension protocol, and shared view commit latency.
- [ ] Expand the architecture guards to reject extension-specific frontend code, extension-owned Web assets, undeclared shell commands, overbroad Tauri permissions, and release artifacts that diverge from the validated distribution inventory.
- [ ] Rewrite `tooling/release` around Tauri bundles and target-triple sidecars. Remove all assumptions about the deleted desktop binary.
- [ ] Package only the Tauri desktop application, CLI, and validated built-in extension executables.
- [ ] Sign Windows artifacts and sign, notarize, and staple macOS artifacts with release credentials.
- [ ] Complete clean-profile first run, summon, settings, actions, diagnostics, rollback, and removal acceptance on every release platform.

## Deferred until required

- [ ] Define post-release settings and database migration policy only when a released format first requires compatibility.
- [ ] Define machine overrides and secret handling before a capability requires them.
- [ ] Add pre-migration maintenance snapshots only before the first post-release destructive schema change.
- [ ] Add captured child output and launched-action process-tree cancellation only when a capability requires them.
- [ ] Define a platform-neutral paste-to-foreground host service with Windows and macOS adapters.
- [ ] Remove the ACP fixture from release packaging before 1.0.
