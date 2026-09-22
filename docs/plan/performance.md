# Performance: Open Validation

Current implementations and Criterion targets live with their owning modules. Prior component measurements are not end-to-end acceptance evidence. Re-measure against the current code before making a performance claim.

## Targets

- Warm summon to focused interactive overlay: P95 at or below 50 ms.
- Input or navigation event to visual update: P95 at or below 16.7 ms.
- Committed query to coherent result DOM: P95 at or below 50 ms.
- Stable 60 Hz frame pacing and 120 Hz where supported.
- No frontend task above 50 ms during summon, typing, navigation, or scrolling.
- Hidden idle: no frontend polling or animation loop, with near-zero CPU.
- No filesystem, SQLite, image decode, or blocking extension work on the Tauri event loop or WebView main thread.

These are acceptance targets, not achieved results.

## Desktop benchmark to build

A Rust controller must start the unchanged release-equivalent Nanika application and generate 1,000- and 2,000-entry workloads through its real extension process, runtime, Tauri Channel, system WebView, and production frontend. Measure total startup separately from complete-list rendering. Exercise search, clear, repeated summon and dismiss, navigation, scrolling, icon completion, and hidden idle.

Record process-start-to-interactive, input-to-visible-results, frame intervals, process-tree CPU and memory, frontend heap and DOM size, JavaScript and CSS bytes, parse/evaluation time, and extension/IPC/Isolation overhead. Include cold and warm cache states, build profile, commit and lockfile hashes, WebView version, reference-machine profile, sample counts, thresholds, and raw samples under `target/performance`. Do not ship instrumentation in production artifacts.

Use at least 200 warm summons and 1,000 query updates for release evidence; shorter runs validate the harness only. Compare named baselines on the same machine. `just check` smoke-checks Criterion targets; use `cargo bench --workspace --locked` for component measurements. A component benchmark cannot establish input-to-visible-results latency.

## Platform acceptance

Run the release-equivalent application on physical Windows 10+ with WebView2 and macOS 13+ with WKWebView. Validate high and mixed DPI, IME, focus, active-monitor placement, hidden idle, failures, and 60 Hz/120 Hz behavior where available. Use native profilers and computer-use for visible outcomes. Report Windows and macOS results separately, including unvalidated cases.
