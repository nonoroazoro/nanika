# Nanika Tasks

Open work only. The code defines current behavior. Remaining design is in [technical stack](tech-stack.md), [platform architecture](platform-architecture.md), [UI](ui.md), and [performance](performance.md). Release gates are in [release](release.md).

## Extension-first completion

- [ ] Derive and verify the packaged built-in inventory in signed release artifacts.
- [ ] Complete zero-extension startup, partial failure, explicit process-exit, disablement, request-correlation, and independent-host contract tests.
- [ ] Audit shell, frontend, engine, and storage boundaries for capability-specific branches. Move capability behavior into extensions or replace presentation-only branching with protocol metadata.
- [ ] Deliver invocation completion, streaming output, extension view updates, configuration application results, diagnostics, and runtime state through session-bound Tauri channels.
- [ ] Add architecture checks rejecting domain implementations outside `apps/extensions`, extension-owned frontend code/assets, Tauri dependencies in `engine`, removed top-level layouts, undeclared shell commands, overbroad permissions, and release inventory mismatches.

## Desktop shell

- [ ] Validate lifecycle, input and IPC behavior in the actual Tauri application on Windows and macOS, including high-DPI and hidden-idle measurements.
- [ ] Validate startup enablement, settings-window lifecycle and stale-instance handling in the actual application on both platforms.
- [ ] Add frontend readiness, window visibility, focus, and interactive activation milestones while retaining passive native hotkey delivery timing.
- [ ] Verify permitted and rejected Isolation command envelopes in the actual Tauri application.
- [ ] Validate the packaged Isolation policy with small/large/small Channel delivery on WKWebView and WebView2 and measure IPC overhead.
- [ ] Validate transparent-window startup, borders, shadows, focus, and active-monitor placement on physical Windows and macOS systems.
- [ ] Evaluate stable native window effects only as measured progressive enhancement with a complete semantic CSS fallback.

## Frontend architecture and design system

- [ ] Expand semantic tokens for typography, color, spacing, size, radius, elevation, motion, and interaction states using plain CSS.
- [ ] Implement shared Svelte primitives for SearchInput, ResultList, ResultRow, SectionHeader, StatusBar, KeyHint, DetailPanel, EmptyState, LoadingState, and DiagnosticState.
- [ ] Add a separately scoped error boundary and explicit asynchronous error handling around extension route content.
- [ ] Add bundled typed message catalogs selected from the operating-system locale with deterministic English fallback.
- [ ] Keep every matching result accessible. Measure the complete 1,000- and 2,000-application catalogs before choosing list virtualization.
- [ ] Define coherent loading, empty, degraded, actionable error, and unavailable states before declaring a surface complete.
- [ ] Add an explicit accessible Root Search result-count announcement.

## Testing

- [ ] Test Rust request validation and serialized contract shapes; verify frontend integration in the actual Tauri application.
- [ ] Use computer-use in the actual Tauri application to verify Root Search typing, IME boundaries, Enter, Up and Down clamping, boundary-only scrolling, pointer activation, stable snapshots, reopen selection, icon completion, and pressed-state release.
- [ ] Verify application full pinyin, initials, common polyphonic names, mixed Han/Latin queries, cross-name search, exact-match precedence, and short-fragment false positives in the actual Tauri application on both supported platforms.
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
- [ ] Create `tooling/release` around the Tauri bundle and target-triple sidecar boundaries.
- [ ] Package only the Tauri desktop application, CLI, and validated built-in extension executables.
- [ ] Sign Windows artifacts and sign, notarize, and staple macOS artifacts with release credentials.
- [ ] Complete clean-profile first run, summon, settings, actions, diagnostics, rollback, and removal acceptance on every release platform.

## Deferred until required

- [ ] Define post-release configuration and database migration policy only when a released format first requires compatibility.
- [ ] Define machine overrides and secret handling before a capability requires them.
- [ ] Add pre-migration maintenance snapshots only before the first post-release destructive schema change.
- [ ] Add captured child output and launched-action process-tree cancellation only when a capability requires them.
- [ ] Define a platform-neutral paste-to-foreground host service with Windows and macOS adapters.
- [ ] Remove the development ACP fixture before 1.0.
