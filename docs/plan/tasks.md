# Nanika Tasks

Status: Tauri is the only pre-1.0 desktop baseline. Every unchecked item is a TODO.

## Product invariants

- The release baseline is macOS 13+ and Windows 10+ only. Platform-specific implementations belong behind typed adapters and target-specific packaging; unsupported platforms must fail explicitly rather than consume a fallback.

- Extensions are the only first-class domain capability unit.
- The bare host owns infrastructure, orchestration, shared control-plane surfaces, and presentation contracts. It contributes no domain candidate or action.
- Built-in and external extensions use the same complete manifest schema, validation, process, protocol, permission, view, action, failure, and diagnostics paths. Built-in identity is the only distribution-owned distinction.
- Built-in status is host-owned distribution metadata. It grants no runtime privilege.
- Rust owns search, storage, configuration, diagnostics, extension supervision, host services, and platform integration.
- Tauri owns the desktop shell. Svelte 5, TypeScript, Vite, and plain CSS own presentation and local interaction.
- Extensions provide bounded data and typed actions. They never provide frontend code, HTML, CSS, scripts, components, remote UI, DOM access, or Tauri access.
- There is no compatibility layer, parallel UI, or migration path for the unpublished renderer and pre-release schemas.
- There are no hidden product watchdogs, retries, restarts, result caps, retention jobs, destructive recovery paths, or silent fallbacks. Clipboard history is the only current retention policy: its distribution contribution declares configurable count and age limits. Hard trust-boundary validation rejects explicitly. Accepted work is not dropped.

## Completed foundation

- [x] Reorganize the repository by product responsibility under `apps`, `engine`, and `tooling`.
- [x] Move the CLI to `apps/cli`, shipping extensions to `apps/extensions/built-in`, protocol fixtures to `apps/extensions/fixtures`, shared behavior to `engine`, and repository support to `tooling`.
- [x] Remove the top-level `crates`, `extensions`, `scripts`, `packaging`, and root `dist` layouts.
- [x] Remove every superseded desktop presentation path, native integration duplicate, test, benchmark, asset, and dependency.
- [x] Make the current database schemas the only pre-release development baseline.
- [x] Create `apps/desktop/frontend` and `apps/desktop/shell` as separate presentation and privileged desktop boundaries.
- [x] Pin the current Active LTS Node.js line and exact mutually compatible stable frontend and Tauri tool versions.
- [x] Configure Svelte 5, TypeScript, Vite, pnpm, `svelte-check`, ESLint through `eslint-config-zoro`, and dprint without SvelteKit.
- [x] Create the Tauri 2 shell with a hidden launcher window, explicit capabilities, command pruning, restrictive CSP, Isolation Pattern, disabled asset protocol, tray ownership, global shortcut ownership, single-instance activation, and active-monitor placement.
- [x] Move search, extension supervision, storage, and permission-checked host services behind the UI-independent `engine/runtime` service.
- [x] Give every built-in its own ordinary `manifest.jsonc` and embed the reviewed manifest set as the Host-owned development inventory.
- [x] Connect Root Search through bounded Rust DTOs, explicit Tauri commands, a session-bound Tauri channel, and one typed frontend bridge.
- [x] Make query commands acknowledgement-only and deliver initial results, background discovery updates, and search phases through one Channel per page lifetime.
- [x] Add session/request/revision checks, a sole delivery worker, one in-flight message with receive acknowledgement, latest-state coalescing, and explicit delivery failure diagnostics without acknowledgement expiry.
- [x] Allow Tauri's exact internal large-Channel fetch through Isolation without broadening application or plugin permissions.
- [x] Add a validated `nanika-icon` custom protocol backed by a bounded off-event-loop reader and immutable extension-scoped cache identities.
- [x] Add the initial semantic combobox/listbox implementation with native text editing, clamped keyboard selection, pointer activation, and query selection on WebView mount.
- [x] Replace the repository quality entry points with `tooling/quality` checks for Rust, frontend formatting, linting, type analysis, production builds, and initial architecture boundaries.
- [x] Make `just dev` build the latest debug extension binaries before Tauri starts, keep one reusable development target, and make `just check` use and delete a unique temporary Cargo target.
- [x] Add static `contributes.configuration` data, host-owned complete-snapshot validation, comment-preserving atomic JSONC persistence, configuration-bearing Nanika initialization, request-correlated live application results, and ACP `session/new` metadata.
- [x] Add Clipboard retention configuration with defaults of 50 entries and 7 days, applied transactionally at startup, after capture, and after a live configuration update.
- [x] Make Application configuration acknowledgement wait for the configuration-driven discovery scan and restore its prior in-memory configuration on queue or scan failure.

## Extension-first completion

- [x] Add ordinary manifests for every bundled extension and derive the development inventory from validated manifest data.
- [x] Stage and declare built-in executables through Tauri `bundle.externalBin` with target-triple filenames.
- [ ] Derive and verify the packaged built-in inventory in signed release artifacts.
- [x] Reject external packages that attempt to assert built-in identity or replace a reserved built-in extension.
- [ ] Complete zero-extension startup, partial failure, explicit process-exit, disablement, request-correlation, and independent-host contract tests.
- [ ] Audit shell, frontend, engine, and storage boundaries for capability-specific branches. Move capability behavior into extensions or replace presentation-only branching with protocol metadata.
- [ ] Deliver invocation completion, streaming output, extension view updates, configuration application results, diagnostics, and runtime state through session-bound Tauri channels.
- [x] Render extension List, Split, Detail, filter, pagination, nested navigation, Back, and typed actions through shared Svelte components.
- [x] Route every extension view action through Rust authorization and the versioned protocol to its owning extension.
- [ ] Expose the implemented runtime configuration registry through a Settings window generated from bounded `contributes.configuration` data, then add the Settings tray action.
- [ ] Add architecture checks that reject domain implementations outside `apps/extensions`, extension-specific frontend components, extension-owned Web assets, Tauri dependencies in `engine`, and removed top-level layouts.

## Desktop shell

- [x] Make runtime initialization failures visible through a bounded frontend diagnostic instead of logging only.
- [x] Consume invocation navigation effects without polling and preserve or dismiss the launcher according to the typed effect.
- [ ] Complete startup enablement, settings-window lifecycle, shutdown coordination, and explicit stale-instance handling through Tauri and platform adapters.
- [ ] Add frontend readiness, window visibility, focus, and interactive activation milestones while retaining passive native hotkey delivery timing.
- [ ] Verify permitted and rejected Isolation command envelopes in the actual Tauri application.
- [ ] Validate the packaged Isolation policy with small/large/small Channel delivery on WKWebView and WebView2 and measure IPC overhead.
- [ ] Validate transparent-window startup, borders, shadows, focus, and active-monitor placement on physical Windows and macOS systems.
- [ ] Evaluate stable native window effects only as measured progressive enhancement with a complete semantic CSS fallback.
- [x] Keep the implemented tray limited to Open Nanika and Quit. Add Settings only with the Settings surface; Application refresh remains an Application Extension action.

## Frontend architecture and design system

- [ ] Implement the design principles and interaction rules in `ui.md` without redefining behavior inside feature components.
- [ ] Expand semantic tokens for typography, color, spacing, size, radius, elevation, motion, and interaction states using plain CSS.
- [ ] Implement shared Svelte primitives for SearchInput, ResultList, ResultRow, SectionHeader, StatusBar, KeyHint, DetailPanel, EmptyState, LoadingState, and DiagnosticState.
- [x] Keep Tauri imports inside the typed bridge. Components consume application services and typed snapshots only.
- [x] Use Svelte 5 runes and current event syntax. Use `$derived` for derived state and `$effect` only for external synchronization.
- [x] Add `<svelte:boundary>` around the application root and handle search startup, delivery, and command failures explicitly.
- [ ] Add a separately scoped error boundary and explicit asynchronous error handling around the already implemented extension route content.
- [x] Prohibit `{@html}` for application and extension data.
- [ ] Add bundled typed message catalogs selected from the operating-system locale with deterministic English fallback.
- [x] Render localized application titles while preserving original names as search aliases.
- [x] Connect keyboard selection to native `scrollIntoView` with nearest alignment; keep browser scrolling as the source of truth without custom geometry or a parallel scroll model.
- [ ] Keep every matching result accessible. Measure the complete 1,000- and 2,000-application catalogs before choosing list virtualization.
- [ ] Define coherent loading, empty, degraded, actionable error, and unavailable states before declaring a surface complete.
- [x] Remove the extension StatusBar when the current view has no actions.
- [ ] Add an explicit accessible Root Search result-count announcement.
- [x] Acquire macOS application icons through `NSWorkspace`, draw them into sRGB pixels, and share alpha-only cropping and cache normalization with Windows Shell icons. Render native icons, host-owned contribution artwork, content-type artwork, and transparent fallbacks in 26 CSS px slots without additional tiles or masks. Publish metadata before icon work, populate application icons in ten-item batches, and paginate Clipboard history in automatic ten-item pages without narrowing search scope.
- [x] Give Clipboard History a compact shared search header, selected-over-hover precedence, resume synchronization, and visually distinct text, file, and image detail presentations.

## Testing

- [x] Keep frontend build and static-analysis packages development-only.
- [ ] Test Rust request validation and serialized contract shapes; verify frontend integration in the actual Tauri application.
- [x] Test configuration schema validation, standard integer `multipleOf` semantics, atomic persistence, runtime result correlation, ACP metadata, Clipboard retention, and Application candidates immediately after configuration acknowledgement.
- [ ] Use computer-use in the actual Tauri application to verify Root Search typing, IME boundaries, Enter, Up and Down clamping, boundary-only scrolling, pointer activation, stable snapshots, reopen selection, icon completion, and pressed-state release.
- [ ] Use computer-use to verify shared components through semantic roles, accessible names, keyboard and pointer input, focus, rendered state, themes, reduced motion, and failures.
- [ ] Use computer-use to verify extension presentation and actions through the actual extension-to-UI path.
- [ ] Record visual acceptance for stable shared primitives in the actual Tauri application.
- [ ] Exercise release-equivalent Tauri applications with computer-use on WebView2 and WKWebView.
- [ ] Record physical acceptance for cold-start app results, repeated search/clear, page reload, delayed discovery, and explicit transport failure presentation. Do not infer acceptance from a Rust send log or unit test.
- [ ] Validate all supported behavior on physical Windows and macOS systems, including high DPI, mixed DPI, CJK IME, accessibility scaling, 60 Hz, and 120 Hz.
- [ ] Audit every asynchronous boundary for dropped work, undocumented caps, watchdogs, automatic retries or restarts, silent fallback, implicit deletion, suppressed errors, uncorrelated responses, and mismatched messages that are silently ignored. Keep only documented trust-boundary limits and idempotent latest-state coalescing.
- [ ] Add a separately authorized repair operation for an explicitly detected interrupted package transaction; keep same-operation compensation synchronous and visible.

## Performance and release

- [ ] Build a Rust performance controller that starts the unchanged Nanika application and drives a Rust-generated 1,000- and 2,000-entry workload through the real extension, runtime, Tauri Channel, system WebView, and production frontend.
- [ ] Produce the actual Nanika performance report: total startup, full-list rendering, search/clear, scrolling frame intervals, icon costs, and process-tree resource use. Include environment, cache state, sample counts, and a comparison table.
- [ ] Add Tauri summon, first-paint, focus, input-to-results, scrolling, memory, and hidden-idle measurements.
- [ ] Record JavaScript and CSS bytes, chunk count, source-map absence, parse time, and evaluation time for production frontend builds.
- [ ] Measure channel, Isolation Pattern, icon protocol, extension protocol, and shared view commit latency.
- [ ] Expand the architecture guards to reject extension-specific frontend code, extension-owned Web assets, undeclared shell commands, overbroad Tauri permissions, and release artifacts that diverge from the validated distribution inventory.
- [ ] Create `tooling/release` around the implemented Tauri bundle and target-triple sidecar boundaries.
- [ ] Package only the Tauri desktop application, CLI, and validated built-in extension executables.
- [ ] Sign Windows artifacts and sign, notarize, and staple macOS artifacts with release credentials.
- [ ] Complete clean-profile first run, summon, settings, actions, diagnostics, rollback, and removal acceptance on every release platform.

## Deferred until required

- [ ] Define post-release configuration and database migration policy only when a released format first requires compatibility.
- [ ] Define machine overrides and secret handling before a capability requires them.
- [ ] Add pre-migration maintenance snapshots only before the first post-release destructive schema change.
- [ ] Add captured child output and launched-action process-tree cancellation only when a capability requires them.
- [ ] Define a platform-neutral paste-to-foreground host service with Windows and macOS adapters.
- [ ] Remove the ACP fixture from release packaging before 1.0.
