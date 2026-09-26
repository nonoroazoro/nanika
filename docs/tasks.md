# TODO

Open work only. Remove completed, abandoned or superseded items. Code/tests define
what exists; optional design items require a concrete product need before implementation.
See [architecture](platform-architecture.md), [lifecycle](extension-lifecycle.md) and
[design system](design-system.md).

## Implementation gaps

- [ ] Add an explicit repair operation for interrupted extension package transactions;
      preserve committed data and report unresolved conflicts. No automatic startup repair.
- [ ] Scope extension rendering failures to their route. The current launcher-wide
      boundary exists; a failing extension surface should not require reloading Root Search.
- [ ] Add an accessible Root Search result-count announcement and validate loading,
      empty and degraded result states with a screen reader.

## Native acceptance

- [ ] Validate macOS 13 WKWebView: launcher focus/input/IME, Settings, menus, Finder
      reveal, shared scrollbars, reduced motion and hidden-state settling.
- [ ] Validate macOS live enable/disable/recovery, current-query reactivation and open
      route retirement. Check withdrawal, process/descendant exit and readiness separately.
- [ ] Validate macOS viewport delivery with 50,000 candidates: fast scrolling, keyboard
      navigation and selection during background root commits.
- [ ] Validate Windows minimum supported WebView, mixed DPI/text scaling, physical CJK
      IME and screen-reader behavior. Recheck caption fades and scrollbars at native DPI.
- [ ] Validate login startup, second-instance activation, stale instances, shortcuts,
      monitor placement and shutdown on both platforms.
- [ ] Validate process containment, atomic replacement failures, package permissions,
      diagnostics opening and directory picker ownership/cancellation on both platforms.
- [ ] Validate native app discovery/activation, icon reuse, clipboard revisions/file
      thumbnails and search with pinyin, initials, polyphonic and mixed-script inputs.
- [ ] Validate release-equivalent Isolation rejection/acceptance and small/large/small
      Channel delivery on both WebViews, including transport failures and delayed discovery.
- [ ] Measure comparable cold start, summon/focus/readiness, search, IPC/view commits,
      icon/scroll latency and frame pacing at 60/120 Hz. Measure memory and hidden idle;
      browser fixtures and build size do not substitute for native results.

## Release readiness

- [ ] Automate releases and inspect final artifacts: CLI, declared sidecars/resources,
      built-in identities/checksums and exclusion of development tooling/test fixtures.
- [ ] Complete Windows signing and macOS signing, notarization and stapling.
- [ ] Run clean-profile install, first run, Settings/actions, diagnostics, startup and
      removal on each platform; verify package rollback where supported.

## Optional design work

- [ ] Evaluate live install/uninstall. Publish inventory only after package commit;
      finish disable before unregistering/deleting files. Built-in files remain release-owned.
- [ ] Design typed localization catalogs when localization is scheduled, using OS
      locale and a deterministic English fallback.
- [ ] Define session-bound streaming output/diagnostics when an extension surface
      requires it, including launched-process output ownership and cancellation.
- [ ] Design paste-to-foreground only for an approved workflow, with Windows/macOS
      adapters and explicit focus/clipboard ownership.
