# Nanika Reference Tasks

Unfinished candidates and validation gaps, not a roadmap or a commitment.
Reassess relevance and scope before starting an item; code and tests define what
already exists. Remove completed or abandoned items instead of keeping a history.
Design references: [extensions](extension-lifecycle.md),
[architecture](platform-architecture.md), and [design system](design-system.md).

## Product and architecture candidates

- Live extension enable/disable with graceful shutdown, instance-scoped cleanup
  and Settings controls, following the extension lifecycle proposal. Deferred
  until the foundation is committed and implementation is separately authorized.
- Session-bound frontend delivery for streaming output, diagnostics and runtime
  state where a product surface needs them.
- Additional automated boundary checks for domain code outside extensions,
  extension-owned frontend assets and overly broad shell permissions.
- An explicit repair operation for interrupted package transactions.
- Immediate-application Switch settings and an audit of visual/motion parameters
  against applicable Fluent 2 sources, documenting local adaptations.
- A route-scoped extension error boundary and local asynchronous failure handling.
- Typed message catalogs using OS locale and a deterministic English fallback.
- An accessible Root Search result-count announcement, plus remaining gaps in
  loading, degraded, unavailable and actionable-error presentation.
- Optional native window effects, subject to measured benefit and a legible CSS baseline.

## Validation and measurement gaps

- Actual macOS 13 WKWebView acceptance, including native focus, menus, Settings,
  Finder reveal and extension interaction. Existing Windows checks do not cover it.
- Windows minimum-WebView and mixed-DPI acceptance; physical CJK IME, screen readers,
  text scaling, live OS reduced motion and 60/120 Hz behavior on both platforms.
- Remaining native lifecycle acceptance: launch at login, second-instance
  activation, stale instances, global shortcuts, monitor placement and shutdown.
- Native data-path acceptance: extension process-tree containment, atomic file
  replacement failure, package target/permissions, diagnostics opening, directory
  picker ownership/cancellation, application discovery/activation, icon cache reuse,
  clipboard revisions and file thumbnails.
- Release-equivalent packaged Isolation checks for accepted/rejected envelopes and
  small/large/small Channel delivery on both WebViews.
- Native regression coverage for cold start, delayed discovery, transport failure,
  extension-to-UI actions, and pinyin/initials/polyphonic/mixed-script search.
- A repeatable full-application workload for 1,000/2,000 results before deciding
  whether virtualization is needed. Keep all matching results accessible.
- Comparable startup, summon, first-paint/focus/readiness, search, IPC, view commit,
  icon, scrolling, memory and hidden-idle measurements. Include parse/evaluation
  cost and frame pacing; existing build-size measurements are not runtime evidence.

## Release candidates

- Release automation and final artifact inventory verification, including CLI,
  approved sidecars/resources and exclusion of development tooling. Existing Tauri
  sidecar build/staging does not establish signed-release contents.
- Windows signing and macOS signing, notarization and stapling; verify packaged
  built-in identities and checksums against the signed release.
- Clean-profile acceptance for installation, first run, Settings, actions,
  diagnostics, startup and removal on each release platform, plus failure
  rollback for package transactions where that contract exists.

## Revisit only with a concrete need

- Post-release schema migration, backup and rollback policy when a released format changes.
- Machine overrides and secret handling when a capability needs them.
- Captured launched-process output and action process-tree cancellation.
- A platform-neutral paste-to-foreground service with Windows and macOS adapters.
- Removal of the development ACP fixture before a production release.
