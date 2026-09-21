# Performance Validation

## 2026-09-18 implementation review measurements

The Windows shell regression serializes the same 2,000 synthetic results and a 96,000-byte detail through Tauri Channel. A navigation-only busy update previously included a 486,170-byte full payload; omitting unchanged results and the view document produces 166 bytes. Both sizes use the actual Channel serializer. This measures payload size, not renderer latency, IPC round-trip time, CPU or application memory.

The real extension-process fixture compares explicit `startup` and `onDemand` policies for the same static command. Before invocation, process initialization counts are 1 and 0 respectively. Search publication, preparation and a dormant configuration acknowledgement leave the on-demand process unstarted. Its first invocation initializes the process and persists one usage record. This is controlled lifecycle evidence; full-tree memory, startup distributions and first-activation latency in the actual application remain unmeasured. Existing built-in activation defaults are unchanged.

Windows/macOS Tauri interaction, high-DPI rendering, hidden-idle CPU and native Quit/descendant cleanup remain separate runtime acceptance items.

Performance validation covers the two supported release targets: macOS 13+ and Windows 10+. Shared measurements and user-visible budgets must remain comparable, while platform-specific sampling or instrumentation belongs behind the platform adapter for that target. Unsupported platforms are not substituted as fallback validation targets.

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

| Profile           | Minimum record                                                                                                         |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Windows reference | Windows 10 22H2, x86-64 CPU, 16 GiB RAM, mixed-DPI dual monitors, exact WebView2 runtime version                       |
| macOS reference   | macOS 13 or later, Apple silicon, 16 GiB RAM, Retina display plus one external display, exact operating-system version |

## Deterministic Rust benchmarks

`just check` builds and executes every Criterion target in test mode as a correctness smoke check. Full measurements use `cargo bench --workspace --locked`; the repository currently provides the optional named-baseline wrapper only as `tooling/quality/benchmark.ps1` on Windows. Criterion covers ranking, query delivery, runtime foundation startup, application indexing, extension process activation, calculator evaluation, and clipboard persistence. UI rendering is not benchmarked through Criterion. Use a named baseline only for comparisons on the same reference machine.

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
- configuration save-to-acknowledgement latency, separating atomic persistence, queue admission, extension work, and the correlated terminal result;
- continuous keyboard navigation and scrolling through complete 1,000- and 2,000-application catalogs;
- icon request, decode, cache hit, and visible presentation timing;
- native window-effect compositor cost, startup flash, and fallback behavior when an effect is enabled;
- heap size, DOM node count, event-listener count, long tasks, and animation frame variance;
- hidden-idle timers, animation frames, CPU, memory, process count, and thread count.

Performance instrumentation must not ship in production artifacts. The implemented development monitor uses the native Web Performance API inside the actual WebView. The desktop controller and schema-versioned `target/performance` report writer are not implemented yet; when added, reports must record the commit, worktree state, Rust and frontend lockfile hashes, application hash, machine profile, WebView version, parameters, thresholds, and raw samples.

Use computer-use to exercise UI interactions and rendering in the actual Tauri application using WebView2 on Windows and WKWebView on macOS. Synthetic application catalogs must pass through extension processes, Rust search, and Tauri IPC before reaching the shared frontend. Record total process-start-to-interactive time and separate runtime initialization, result delivery, DOM update, and frame timing. Isolated component timings are not end-to-end application results.

## Development monitor

During `corepack pnpm --dir apps/desktop dev`, press `F9` while the Nanika window is focused and visible to toggle an independent frontend overlay. F5 is reserved for Root Search refresh. Sampling starts disabled. Losing window focus, hiding the document, or disposing the component cancels its animation-frame callback and detaches its search observation listener. Resuming starts a fresh sample window and excludes the hidden interval.

The overlay reports estimated FPS, P95 and maximum frame intervals over the most recent 120 positive `requestAnimationFrame` intervals. It refreshes frame statistics at most every 250 ms. This rolling window is an explicit diagnostic sampling policy, not retained benchmark history. The active sampler adds work even on an otherwise static page; its readings include monitoring overhead.

Search timing follows the current request from the frontend input handler through an accepted ready Channel snapshot, Svelte DOM commit, and two frame callbacks. The last timing estimates a rendering opportunity, not physical display presentation. Superseded requests do not produce completion samples. Rejected queries and explicit failures remain labeled, without measurement deadlines. Observations contain only request identifiers, stages, and timestamps; they do not retain query text or results.

The monitor is loaded only through Vite's development branch. Production builds reject emitted chunks containing development-monitor modules, including builds accidentally run with a development `NODE_ENV`. The overlay, its CSS, keyboard shortcut, and search observation hooks must be absent from production assets. Use platform profilers for release acceptance and compositor measurements.

## Activation trace status

The implemented trace records native hotkey delivery delay through Carbon `EventTime` on macOS and `MSG.time` on Windows when available. Missing native timing omits that observation instead of substituting callback time. Correlation through Rust event handling, active-monitor placement, Tauri visibility, frontend visibility acknowledgement, input focus, and interactive readiness is not implemented yet.

The future frontend readiness acknowledgement must occur only after the current view model is committed, layout has completed, and the search input owns focus. Visibility alone is not interactive readiness. When the complete trace exists, slow activations above 50 ms are warning-level diagnostics. Verbose diagnostics may retain timing values but never query text or clipboard content.

## Planned desktop black-box benchmark

Build the Tauri desktop application in release mode. A Rust controller must launch and observe that real application rather than a component page or browser-test harness. It verifies hidden startup, hidden-idle resource use, repeated summon and dismissal, focus ownership, first interactive paint, and warm activation P50, P95, P99, maximum, and raw samples.

Use at least 200 warm summons and 1,000 query updates for release evidence. Shorter runs validate the harness only. Synthetic input is valid only after the harness proves that the platform receives it through the same production input path.

Rust tests may use Tauri's mock runtime where native behavior is irrelevant. UI acceptance uses computer-use to drive a release-equivalent Tauri application on Windows and macOS against WebView2 and WKWebView. Record actual application logs and rendering measurements; Rust test results do not substitute for desktop acceptance.

Acceptance starts with the first clean Tauri release build.

## macOS system-icon acquisition evidence

On macOS 26.4.1 (Apple silicon), an optimized acquisition probe tested 89 application bundles with `NSWorkspace.iconForFile` and 256 px sRGB drawing. The first measured pass accumulated 1,821.8 ms, with a per-icon median of 16.71 ms and P95 of 42.68 ms. The second pass took 403.2 ms; four subsequent fully warmed passes took 52.3 to 58.3 ms per 89 applications, with a median of 0.596 ms and P95 of 0.725 ms per icon. OS caches were not cleared, so the first pass is not a cold-boot result.

For context, release-optimized raw-resource decoding succeeded for 87 of those applications and accumulated 160.5 ms in its first pass. It does not produce equivalent system-styled images. These acquisition-only measurements exclude metadata discovery, normalization, PNG encoding, persistent-cache writes, database work, and frontend rendering; they do not establish end-to-end startup performance.

The implemented Rust adapter was also exercised in the debug build with an empty isolated application database and PNG cache. It indexed 88 applications with no scan warnings or icon failures: metadata scanning took 1,264.2 ms, followed by 3,798.8 ms to acquire, normalize, encode, and write all three PNG sizes. A second scan took 528.8 ms and its complete-cache population pass took 0.69 ms. These are single-pass observations with existing OS caches, not latency distributions or Tauri startup measurements. Eight matching application samples produced 128 px cached images pixel-identical to the native reference drawings after the same normalization.

The application keeps metadata publication ahead of icon population, generates all missing cache sizes from one macOS working image, and bypasses acquisition on complete persistent-cache hits. The Host sends its final ranked first ten entry IDs as a non-blocking preparation hint. Missing icons for those entries are processed first; remaining icons continue in batches of ten, and each completed batch is published. The WebView lazily fetches and asynchronously decodes result images. First-time cache generation can take seconds, but it never gates Root Search presentation. Validate the complete Rust adapter and actual Tauri display after changes. The native API path compiled with a macOS 13.0 deployment target, but its native visual behavior was tested only on macOS 26.4.1. Windows and older macOS runtime acceptance remain separate requirements.

## Clipboard detail continuity evidence

On 2026-09-19, a visible, focused Windows Tauri development window at 200% scaling was exercised with 20 alternating Up/Down inputs, 100 ms apart. The same live clipboard entries were used before and after the fix. Input used WebView2's [documented local debugging connection](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/debug-visual-studio-code), not a simulated DOM. Temporary DOM and animation-frame observers recorded only counts and node identity, without saving clipboard contents.

| Observation | Before | After |
| --- | --- | --- |
| Selection changes | 20 | 20 |
| Detail article removals | 20 | 0 |
| Frames with no detail article | 7 of 151 | 0 of 152 |
| Original detail article retained | No | Yes |

The previous optimistic-selection mismatch cleared the complete detail subtree while waiting for the extension. The renderer now retains the committed detail, marks the pane busy, and updates it when the Channel supplies the replacement. The observations establish continuity for this exercised Windows scenario, not universal image-decode behavior, 120 Hz acceptance, or macOS correctness. The temporary probe and raw counts are under ignored `target/review-current/detail-flicker*`; no observer or frame loop was added to production code.

## Platform acceptance

Validate on physical Windows and macOS machines:

- focus, IME, candidate-window placement, active-monitor placement, mixed DPI, full-screen applications, and elevated foreground windows;
- global hotkey replacement and conflicts;
- foreground and background second launches;
- startup enable, disable, repair, approval, and hidden idle launch;
- stable Root Search publication while extensions return initial snapshots;
- cold-start empty-query results without typing, input before startup completes, repeated search/clear cycles, stale-result rejection, and an explicit transport error after Channel closure;
- zero-extension startup, built-in and external rendering-path equivalence, and visible behavior while one extension is slow, failed, or disabled;
- saved extension configuration, correlated Nanika live-application success and failure, Application rescan completion before acknowledgement, and ACP next-session application;
- Clipboard resume synchronization after the launcher was hidden, count and age retention, and text, file, and image detail differentiation;
- native text editing, query selection on reopen, listbox navigation, scroll boundary behavior, and pointer activation;
- icon protocol validation, cache hits, decode cost, sharp high-DPI presentation, and scrolling responsiveness;
- 60 Hz and 120 Hz motion, interruption, and reduced motion;
- accessibility roles, active option announcements, keyboard order, and contrast;
- hidden-idle CPU, memory, process count, thread count, timers, and animation frames.

Use Windows Performance Recorder or equivalent ETW tooling on Windows, Instruments on macOS, and WebView developer tooling for frontend traces. Keep raw platform captures out of the repository.

The shared file-icon cache was exercised on macOS in a debug unit test using the test executable. Initial native acquisition, 512 px drawing, alpha-only normalization, and writing 128/512 px PNGs took 1,520.3 ms; an unchanged lookup took 54.2 microseconds. The same test verifies a metadata lookup after constructing a new cache instance, both PNG dimensions, unchanged cache modification time, missing-variant repair, non-empty native pixels, and concrete missing-source errors. This is a single headless CLI observation, not a UI latency distribution; AppKit can return low-alpha template artwork in that environment, so opacity remains part of actual Tauri visual acceptance. Native acquisition runs outside the Tauri/WebView event loops. Clipboard's initial view performs no file-icon I/O; a dedicated background worker prioritizes at most the first three selected-entry paths needed by the bounded collection preview and one path per other visible row, then publishes completed icons through a per-extension coalesced invalidation handled outside Root Search delivery. Windows runtime behavior remains unverified.

A separate debug probe resolved the actual `.pkg` file used for visual comparison through the same shared cache. Its first lookup took 283.6 ms and the subsequent metadata/cache lookup took 0.023 ms. Both 128 and 512 px artifacts were generated; the 512 px image was visually inspected and matches the system package-box artwork. This verifies native extraction and PNG output, not the complete Tauri interaction.

A debug regression probe with 100 visible temporary text files measured the former synchronous all-row icon path at 15,780.6 ms on first use. The former one-selected-icon enrichment phase measured 186.3 ms cold and 0.067 ms warm. The current initial view performs in-memory icon lookup only, returns at most ten matching rows, and delegates metadata lookup, persistent cache resolution, and missing visible icon acquisition to a background worker; those earlier enrichment numbers are retained only as the replaced baseline. File-icon I/O no longer contributes to initial extension presentation or grows with the total retained history. Search and filtering still inspect the complete retained history before the ten-row presentation page is selected.
