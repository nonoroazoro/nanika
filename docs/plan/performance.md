# Performance Validation

Performance results are evidence, not pass or fail gates on ordinary machines. Compare results only on the same hardware, power mode, display topology, operating-system version, WebView version, build profile, and background load.

## Targets

- Warm summon to focused and interactive overlay: P95 at or below 50 ms.
- Text input or navigation event to its next visual update: P95 at or below 16.7 ms.
- Committed query to the first coherent result DOM: P95 at or below 50 ms.
- Stable 60 Hz frame pacing, with 120 Hz support where available.
- No frontend long task above 50 ms during summon, typing, navigation, or scrolling.
- Hidden idle: no animation frame loop, no frontend polling, and near-zero CPU.
- No filesystem, SQLite, image decoding, or blocking extension work on the Tauri event-loop or WebView main thread.
- The Tauri Isolation Pattern remains inside the same input-to-visual and query-to-results budgets; its cryptographic IPC work does not create a separate allowance.

## Reference machines

| Profile | Minimum record |
| --- | --- |
| Windows reference | Windows 10 22H2, x86-64 CPU, 16 GiB RAM, mixed-DPI dual monitors, exact WebView2 runtime version |
| macOS reference | macOS 13 or later, Apple silicon, 16 GiB RAM, Retina display plus one external display, exact operating-system version |

## Deterministic Rust benchmarks

Run the project benchmark command. Criterion covers ranking, query delivery, runtime foundation startup, application indexing and preselection, extension process activation, calculator evaluation, and clipboard persistence. UI rendering is not benchmarked through Criterion. Use a named baseline only for comparisons on the same reference machine.

## Frontend benchmarks

Measure production frontend builds with bundled local assets. Record:

- initial document parse, script evaluation, style calculation, layout, and first paint;
- emitted JavaScript and CSS bytes, chunk count, and production source-map absence;
- warm overlay summon to visible, focused, and interactive state;
- keydown to selected-row update;
- query input to committed result snapshot;
- Isolation Pattern command validation and encryption, plus channel serialization, dispatch, and frontend delivery overhead;
- continuous keyboard navigation and trackpad scrolling through 100 results;
- icon request, decode, cache hit, and visible presentation timing;
- native window-effect compositor cost, startup flash, and fallback behavior when an effect is enabled;
- heap size, DOM node count, event-listener count, long tasks, and animation frame variance;
- hidden-idle timers, animation frames, CPU, memory, process count, and thread count.

Frontend development instrumentation must not ship in production artifacts. Retain schema-versioned JSON reports under `target/performance` with the commit, worktree state, Rust and frontend lockfile hashes, application hash, machine profile, WebView version, parameters, thresholds, and raw samples.

Run pure TypeScript performance-sensitive logic through the Vitest Node project. Run component interaction and rendering checks through Vitest Browser Mode with the Playwright provider and official Svelte renderer in Chromium and WebKit. Browser Mode measurements compare frontend changes under fixed browser versions; they do not replace release-build measurements in WebView2 and WKWebView.

## Activation trace

The activation trace covers native hotkey delivery, Rust event handling, active-monitor placement, Tauri window visibility, frontend visibility acknowledgement, input focus, and interactive readiness. Native timing uses Carbon `EventTime` on macOS and `MSG.time` on Windows when available. Missing native timing marks a sample incomplete instead of substituting callback time.

The frontend emits a readiness acknowledgement only after the current view model is committed, layout has completed, and the search input owns focus. Visibility alone is not interactive readiness. Slow activations above 50 ms are warning-level diagnostics. Verbose diagnostics may retain timing values but never query text or clipboard content.

## Desktop black-box benchmark

Build the Tauri desktop application in release mode. Platform harnesses must verify hidden startup, hidden-idle resource use, repeated summon and dismissal, focus ownership, first interactive paint, and warm activation P50, P95, P99, maximum, and raw samples.

Use at least 200 warm summons and 1,000 query updates for release evidence. Shorter runs validate the harness only. Synthetic input is valid only after the harness proves that the platform receives it through the same production input path.

Unit and component tests may use Tauri's mock runtime where native behavior is irrelevant. Desktop E2E must drive a release-equivalent Tauri application on both Windows and macOS against WebView2 and WKWebView. A harness that supports only one of the two platforms is insufficient; direct `tauri-driver` coverage alone does not satisfy the macOS requirement. Any embedded WebDriver server or test-access plugin is restricted to a test build and must be absent from production binaries and release artifacts.

Acceptance starts with the first clean Tauri release build.

## Platform acceptance

Validate on physical Windows and macOS machines:

- focus, IME, candidate-window placement, active-monitor placement, mixed DPI, full-screen applications, and elevated foreground windows;
- global hotkey replacement and conflicts;
- foreground and background second launches;
- startup enable, disable, repair, approval, and hidden idle launch;
- stable Root Search publication while extensions return initial snapshots;
- native text editing, query selection on reopen, listbox navigation, scroll boundary behavior, and pointer activation;
- icon protocol validation, cache hits, decode cost, sharp high-DPI presentation, and scrolling responsiveness;
- 60 Hz and 120 Hz motion, interruption, and reduced motion;
- accessibility roles, active option announcements, keyboard order, and contrast;
- hidden-idle CPU, memory, process count, thread count, timers, and animation frames.

Use Windows Performance Recorder or equivalent ETW tooling on Windows, Instruments on macOS, and WebView developer tooling for frontend traces. Keep raw platform captures out of the repository.
