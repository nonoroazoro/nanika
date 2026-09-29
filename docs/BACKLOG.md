# Backlog

Candidates only, not an implementation plan or source of design authority. Implement
an item only when the user explicitly requests that item or clearly includes it in
the requested scope. General development, review, cleanup or continuation requests
do not authorize backlog work. Do not select or start items autonomously.

Recheck each requested item against current code, tests and the user's latest
requirements before implementation; entries may be stale or already resolved.
Remove completed, abandoned or superseded items. Do not restore earlier designs
from deleted documents or Git history to satisfy an entry.

## Implementation candidates

Capabilities absent from the current implementation. Inclusion does not establish
priority or authorize development.

- [ ] Add an explicit repair operation for interrupted extension package transactions;
      preserve committed data and report unresolved conflicts. No automatic startup repair.
- [ ] Scope extension rendering failures to their route. The current launcher-wide
      boundary exists; a failing extension surface should not require reloading Root Search.
- [ ] Add an accessible Root Search result-count announcement.

## Performance investigations

Measure first. These are evaluation questions, not confirmed defects or commitments
to change the current design. Use comparable workloads for any before/after checks.

- [ ] Measure selection-only `ViewUpdated` serialization and delivery cost, including
      unchanged collection windows and detail chunks. Determine whether the complete
      bounded view warrants optimization before proposing a transport change.
- [ ] Measure per-selection work, serialized payload size, rendered row count and retained
      text memory after extensive browsing. Include long text chunk reads and repeated
      selection changes; quantify remaining growth before proposing further bounds.
- [ ] Measure comparable cold start, summon/focus/readiness, search, IPC/view commits,
      icon/scroll latency and frame pacing at 60/120 Hz. Measure memory and hidden idle;
      browser fixtures and build size do not substitute for native results.

## Regression coverage

Add automated evidence for existing behavior; these entries do not imply that the
implementation is missing or broken.

- [ ] Add focused regression coverage for a nonzero collection total with no delivered
      rows: the renderer must request a window and must not show terminal No results.
      Also cover a genuinely empty collection with total zero.

## Native acceptance

These items require native execution evidence; implementation and unit tests alone
do not establish completion. A pending check is not a confirmed defect.

- [ ] Verify detail scroll position and native text selection during same-record chunk
      appends, layout resizing and delayed or superseded deliveries on both WebViews.
- [ ] Validate Root Search loading, empty and degraded states with a screen reader;
      include result-count announcements once implemented.
- [ ] Validate System handoff on macOS hardware, including native Accessibility/
      Automation prompts, Finder trash across mounted volumes and OS-owned cancellation.
      The launcher must dismiss after submission without execution-result or progress
      notifications. Cross-compilation does not validate these.
- [ ] Validate disruptive System actions on disposable Windows/macOS sessions:
      lock, sleep/resume, display power, logout/restart/shutdown and permanent trash
      deletion. Keep destructive operations out of automated developer-machine tests.
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
- [ ] Validate Windows registered-app install/update/removal and macOS app discovery/
      activation on native systems. Check localized names, icon invalidation and
      disabled-source withdrawal.
- [ ] Validate clipboard revisions/file thumbnails and search with pinyin, initials,
      polyphonic and mixed-script inputs on both supported platforms.
- [ ] Validate release-equivalent Isolation rejection/acceptance and small/large/small
      Channel delivery on both WebViews, including transport failures and delayed discovery.

## Release readiness

Packaging, distribution and installation work to assess when a release is requested.

- [ ] Automate releases and inspect final artifacts: CLI, declared sidecars/resources,
      built-in identities/checksums and exclusion of development tooling/test fixtures.
- [ ] Complete Windows signing and macOS signing, notarization and stapling.
- [ ] Run clean-profile install, first run, Settings/actions, diagnostics, startup and
      removal on each platform; verify package rollback where supported.

## Optional design work

Ideas requiring an explicit product decision before design or implementation.

- [ ] Evaluate live install/uninstall. Publish inventory only after package commit;
      finish disable before unregistering/deleting files. Built-in files remain release-owned.
- [ ] Design typed localization catalogs when localization is scheduled, using OS
      locale and a deterministic English fallback.
- [ ] Design paste-to-foreground only for an approved workflow, with Windows/macOS
      adapters and explicit focus/clipboard ownership.
