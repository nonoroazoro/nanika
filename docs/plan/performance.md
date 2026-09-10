# Performance Validation

Performance results are evidence, not pass or fail gates on ordinary machines. Compare results only on the same hardware, power mode, display topology, operating-system version, WebView version, build profile, and background load.

## Validation responsibilities

- Rust automated tests verify core behavior, concurrency, protocol and storage contracts, and failure isolation. Rust benchmarks measure core latency and resource use with representative workloads.
- End-to-end performance observation runs the actual Nanika application. The measured path includes extension processes, Rust search, Tauri IPC, and UI rendering. Report process-start-to-interactive latency, input-to-visible-results latency, scrolling frame intervals, and process-tree CPU and memory. Component-only measurements cannot establish these results.
- Other UI behavior and visual acceptance use computer-use in the actual application. Retain observations and diagnostic evidence rather than adding a frontend automated test framework.
- Catalog-scale runs use a Rust-controlled 1,000- and 2,000-entry fixture only to create the workload. The unchanged Nanika executable must still start its real extension process, Rust runtime, Tauri IPC, system WebView, and production frontend. Verify that every entry remains accessible and distinguish fresh application data from a warm cache. Report complete-list rendering separately from total startup, and include search, clearing the query, scrolling, and icon costs. State build profile and sample counts, and distinguish process-cold startup from OS-cold startup.

## Targets

- Warm summon to focused and interactive overlay: P95 at or below 50 ms.
- Text input or navigation event to its next visual update: P95 at or below 16.7 ms.
- Committed query to the first coherent result DOM: P95 at or below 50 ms.
- Stable 60 Hz frame pacing, with 120 Hz support where available.
- No frontend long task above 50 ms during summon, typing, navigation, or scrolling.
- Hidden idle: no animation frame loop, no frontend polling, and near-zero CPU.
- No filesystem, SQLite, image decoding, or blocking extension work on the Tauri event-loop or WebView main thread.
- No extension-owned JavaScript context, WebView, DOM subtree, stylesheet runtime, animation loop, or frontend bundle.
- The Tauri Isolation Pattern remains inside the same input-to-visual and query-to-results budgets; its cryptographic IPC work does not create a separate allowance.

## Reference machines

| Profile | Minimum record |
| --- | --- |
| Windows reference | Windows 10 22H2, x86-64 CPU, 16 GiB RAM, mixed-DPI dual monitors, exact WebView2 runtime version |
| macOS reference | macOS 13 or later, Apple silicon, 16 GiB RAM, Retina display plus one external display, exact operating-system version |

## Deterministic Rust benchmarks

Run the project benchmark command. Criterion covers ranking, query delivery, runtime foundation startup, application indexing, extension process activation, calculator evaluation, and clipboard persistence. UI rendering is not benchmarked through Criterion. Use a named baseline only for comparisons on the same reference machine.

## Frontend benchmarks

Measure production frontend builds with bundled local assets. Record:

- initial document parse, script evaluation, style calculation, layout, and first paint;
- emitted JavaScript and CSS bytes, chunk count, and production source-map absence;
- warm overlay summon to visible, focused, and interactive state;
- keydown to selected-row update;
- query input to committed result snapshot;
- Isolation Pattern command validation and encryption, plus channel serialization, dispatch, and frontend delivery overhead;
- small and large Channel payloads on both sides of Tauri's internal direct-delivery threshold, including acknowledgement round-trip latency;
- the single in-flight message limit and latest-state coalescing under rapid input or a stalled consumer, including delivery of the final cleared query;
- extension protocol validation through shared Svelte component commit for declarative views, and user action return through the typed bridge to the owning extension;
- continuous keyboard navigation and scrolling through complete 1,000- and 2,000-application catalogs;
- icon request, decode, cache hit, and visible presentation timing;
- native window-effect compositor cost, startup flash, and fallback behavior when an effect is enabled;
- heap size, DOM node count, event-listener count, long tasks, and animation frame variance;
- hidden-idle timers, animation frames, CPU, memory, process count, and thread count.

Performance instrumentation must not ship in production artifacts. The controller and report writer are Rust repository tooling. Browser-realm timestamps use the native Web Performance API inside the actual WebView. Retain schema-versioned JSON reports under `target/performance` with the commit, worktree state, Rust and frontend lockfile hashes, application hash, machine profile, WebView version, parameters, thresholds, and raw samples.

Use computer-use to exercise UI interactions and rendering in the actual Tauri application using WebView2 on Windows and WKWebView on macOS. Synthetic application catalogs must pass through extension processes, Rust search, and Tauri IPC before reaching the shared frontend. Record total process-start-to-interactive time and separate runtime initialization, result delivery, DOM update, and frame timing. Isolated component timings are not end-to-end application results.

## Development monitor

During `pnpm --dir apps/desktop dev`, use the Performance button or `Cmd/Ctrl+Shift+P` to toggle an independent frontend overlay. Sampling starts disabled. Closing the monitor, losing window focus, hiding the document, or disposing the component cancels its animation-frame callback and detaches its search observation listener. Resuming starts a fresh sample window and excludes the hidden interval.

The overlay reports estimated FPS, P95 and maximum frame intervals over the most recent 120 positive `requestAnimationFrame` intervals. It refreshes frame statistics at most every 250 ms. This rolling window is an explicit diagnostic sampling policy, not retained benchmark history. The active sampler adds work even on an otherwise static page; its readings include monitoring overhead.

Search timing follows the current request from the frontend input handler through an accepted ready Channel snapshot, Svelte DOM commit, and two frame callbacks. The last timing estimates a rendering opportunity, not physical display presentation. Superseded requests do not produce completion samples. Rejected queries and explicit failures remain labeled, without measurement deadlines. Observations contain only request identifiers, stages, and timestamps; they do not retain query text or results.

The monitor is loaded only through Vite's development branch. Production builds reject emitted chunks containing development-monitor modules, including builds accidentally run with a development `NODE_ENV`. The overlay, its CSS, keyboard shortcut, and search observation hooks must be absent from production assets. Use platform profilers for release acceptance and compositor measurements.

## Activation trace

The activation trace covers native hotkey delivery, Rust event handling, active-monitor placement, Tauri window visibility, frontend visibility acknowledgement, input focus, and interactive readiness. Native timing uses Carbon `EventTime` on macOS and `MSG.time` on Windows when available. Missing native timing marks a sample incomplete instead of substituting callback time.

The frontend emits a readiness acknowledgement only after the current view model is committed, layout has completed, and the search input owns focus. Visibility alone is not interactive readiness. Slow activations above 50 ms are warning-level diagnostics. Verbose diagnostics may retain timing values but never query text or clipboard content.

## Desktop black-box benchmark

Build the Tauri desktop application in release mode. A Rust controller must launch and observe that real application rather than a component page or browser-test harness. It verifies hidden startup, hidden-idle resource use, repeated summon and dismissal, focus ownership, first interactive paint, and warm activation P50, P95, P99, maximum, and raw samples.

Use at least 200 warm summons and 1,000 query updates for release evidence. Shorter runs validate the harness only. Synthetic input is valid only after the harness proves that the platform receives it through the same production input path.

Rust tests may use Tauri's mock runtime where native behavior is irrelevant. UI acceptance uses computer-use to drive a release-equivalent Tauri application on Windows and macOS against WebView2 and WKWebView. Record actual application logs and rendering measurements; Rust test results do not substitute for desktop acceptance.

Acceptance starts with the first clean Tauri release build.

## Platform acceptance

Validate on physical Windows and macOS machines:

- focus, IME, candidate-window placement, active-monitor placement, mixed DPI, full-screen applications, and elevated foreground windows;
- global hotkey replacement and conflicts;
- foreground and background second launches;
- startup enable, disable, repair, approval, and hidden idle launch;
- stable Root Search publication while extensions return initial snapshots;
- cold-start empty-query results without typing, input before startup completes, repeated search/clear cycles, stale-result rejection, and an explicit transport error after Channel closure;
- zero-extension startup, built-in and external rendering-path equivalence, and visible behavior while one extension is slow, failed, or disabled;
- native text editing, query selection on reopen, listbox navigation, scroll boundary behavior, and pointer activation;
- icon protocol validation, cache hits, decode cost, sharp high-DPI presentation, and scrolling responsiveness;
- 60 Hz and 120 Hz motion, interruption, and reduced motion;
- accessibility roles, active option announcements, keyboard order, and contrast;
- hidden-idle CPU, memory, process count, thread count, timers, and animation frames.

Use Windows Performance Recorder or equivalent ETW tooling on Windows, Instruments on macOS, and WebView developer tooling for frontend traces. Keep raw platform captures out of the repository.
