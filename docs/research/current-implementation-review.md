# Current implementation review against the launcher research

Date: 2026-09-18.

Status: Findings F1-F8 addressed in the current implementation. F1 was already fixed in the user's commit `286cf35` and was preserved. Explicit static-only on-demand activation is implemented. ACP product work remains deferred.

Reference: [Launcher, extensions, and deferred ACP research](launcher-extension-acp.md).

## Resolution

| Finding | Current implementation | Regression evidence |
| --- | --- | --- |
| F1: preparation feedback | Preserved the existing changed-snapshot gate. | Real runtime preparation-completion and navigation-wake regression. |
| F2: stale invalidation | Completion checks session, extension, view, route and revision. | Reused route/revision across replacement sessions and different owners cannot replace content. |
| F3: destructor-dependent Quit | Tauri exit is deferred until explicit shutdown finishes. Initialization, protocol cancellation, shell operations, delivery, extension workers, native host services, storage, search, icon requests, instance activation and diagnostics have explicit completion paths. | Retained-Arc shutdown and blocked-initialization fixtures. Actual native Quit remains a platform acceptance item. |
| F4: stale keyboard target | Local selection intent drives the highlighted item and action target. Ordered submissions coalesce only adjacent unsent selections. A single input scheduler joins RPC completion receipts with authoritative Channel navigation revisions before continuing queued query/resume intent. | Transport ordering/coalescing, both RPC/Channel arrival orders, exact revision barriers, route replacement, and failure without automatic retry. Actual keyboard/ARIA behavior remains a UI acceptance item. |
| F5: full snapshot retransmission | The initial update is complete; unchanged result lists and current view documents are omitted afterwards. The bridge reconstructs snapshots without copying unchanged sections. Immutable view documents are shared with `Arc`; serialization and send occur outside the state lock. | Reentrant Channel acknowledgement, omitted-section assertions, and payload measurement. |
| F6: successful execution lost with UI | `RuntimeService::invoke_recorded` preserves execution outcome and recording error separately. Navigation or explicit remote view cleanup runs even when recording fails; concurrent navigation and recording errors are both reported. | Successful execution persists once. A real extension and SQLite transaction failure preserve a created view's close contract and roll back history/usage atomically; shell tests cover presentation and cleanup failure reporting. |
| F7: old session blocks new operations | Each session owns its view-operation lock; replacement sessions acquire an independent lock. | New-session lock acquisition while the old lock is held, plus stale-completion rejection. |
| F8: unbounded retained navigation | Explicit admission limit of 32 routes. A rejected newly created remote view is closed; existing routes are retained, and cleanup errors remain concrete. | Overflow retains existing routes and returning one level restores capacity. |

The static contribution catalog now belongs to the search owner and is registered once. It reuses immutable candidate vectors across queries and preserves the current dynamic-extension completion barrier. `activation` defaults to `startup`; explicit `onDemand` is restricted to static Nanika-protocol extensions. Search and preparation do not start those processes, dormant configuration becomes their initialization snapshot, and the first accepted invocation starts them once. Failed activation is terminal for that runtime lifetime. No built-in startup defaults changed. Each enabled extension still has one scheduling thread.

The platform contract and policy are recorded in [platform architecture](../plan/platform-architecture.md), [technical stack](../plan/tech-stack.md), and [tasks](../plan/tasks.md).

## Measurements and validation of the fixes

The actual Tauri Channel serializer produced 486,170 bytes for a full update with 2,000 synthetic results and a 96,000-byte detail, versus 166 bytes for the corresponding navigation-only update. The real child-process fixture observed one initialized process before invocation under `startup`, versus zero under `onDemand`; the first on-demand invocation then initialized and completed successfully. These measurements do not establish application RAM savings, native startup latency or frame pacing. See [performance validation](../plan/performance.md).

Final Windows validation: `just check` passed with 263 Rust tests, 10 frontend transport/scheduling tests, benchmark smoke checks, extension builds/preparation, Rust formatting, Clippy with warnings denied, Rust documentation, dprint, ESLint, Svelte/TypeScript checks and the production frontend build. The build emitted 70.97 kB JavaScript (25.31 kB gzip) and 17.21 kB CSS (3.68 kB gzip). An initial sandboxed run failed the existing Windows product-path discovery test; the complete rerun in the real user environment passed, including that test. Temporary check artifacts stayed under `target`. The complete local log is under ignored `target/review-current/followup-just-check.log`.

No actual Tauri computer-use acceptance or macOS runtime validation has been performed for these changes. Native keyboard/ARIA behavior, Quit, high-DPI rendering and full-tree resource measurements remain acceptance work, not inferred from unit tests. ACP chat, chat persistence and endpoint integration remain deferred. Existing staged research documents were preserved; implementation changes were left unstaged.

## Original review scope and evidence

The following sections preserve the pre-fix audit record. Their defect descriptions and line references describe the reviewed baseline, not the resolved working tree above.

Reviewed the working-tree search, extension scheduling, desktop IPC, view navigation, frontend interaction, and process-lifecycle paths. The working tree changed concurrently during the review; these findings describe the cited source paths inspected, not certification of an immutable commit. HEAD observed near completion was `4afa3be4f7d103852a6d5ec45f6db87846a57efb`.

Storage, configuration, package registration, protocol validation, and Windows/macOS process containment were traced where they intersected those paths. They were not subjected to a complete independent security audit. The ACP prototype was used to distinguish preliminary infrastructure from planned product functionality, not evaluated as a production chat implementation.

`just check` completed successfully on Windows: extension builds/preparation, dprint, ESLint, Svelte/TypeScript checks, production frontend build, Rust formatting, Clippy, workspace tests and benchmark smoke checks, and Rust documentation. The frontend build emitted 67.83 kB JavaScript, 24.19 kB gzip. These are bundle measurements, not application RAM or installed package size.

Additional review-only probes live under ignored `target/review-current`. Four Rust probes include the shipped `SearchDelivery.rs`, `ViewInvalidationDelivery.rs`, and `NavigationState.rs` algorithms unchanged, using controlled runtime/protocol/transport stand-ins. One JavaScript probe executes the extracted `ExtensionView.svelte` key handler after removing its TypeScript annotations, using controlled DOM/state stand-ins. They reproduce the current behaviors below; they are not passing regression tests for the desired behavior and are not full Tauri UI tests.

No actual Tauri UI performance acceptance or macOS runtime validation was performed. No CPU percentage, memory saving, or cross-platform latency improvement is claimed.

## Findings

### F1. P1: Entry preparation creates a self-sustaining delivery wake loop

Evidence: [SearchDelivery.rs:33](../../apps/desktop/shell/src/SearchDelivery.rs#L33), [ExtensionSearchCoordinator.rs:116](../../engine/runtime/src/ExtensionSearchCoordinator.rs#L116), [ExtensionSearchWorker.rs:150](../../engine/runtime/src/ExtensionSearchWorker.rs#L150), and [ExtensionSearchWorker.rs:241](../../engine/runtime/src/ExtensionSearchWorker.rs#L241).

Every delivery wake with a current search snapshot calls `prepare_visible_entries` before checking in-flight delivery or whether anything changed. The coordinator queues preparation for every worker, including workers with no matching entry IDs. Each completed preparation then reaches the worker's unconditional notifier. That notifier wakes delivery again, which schedules the same preparation again. This does not require new input, an icon change, a new snapshot, or a frontend acknowledgement.

The controlled feedback probe reached its deliberate 100-cycle stop with one unchanged snapshot and zero UI acknowledgements. The runtime stand-in models the unconditional worker notification established by the inspected source. This verifies the feedback mechanism, not its production CPU rate. The real native adapter additionally writes a `PrepareEntries` frame on each dispatch.

Correction: Track the last submitted preparation identity, at least generation and relevant ordered entry IDs, and enqueue only changed preparation. Successful scheduling hints that do not change observable state must not issue a general state-change notification. Preserve explicit failure notification and genuine icon/candidate invalidation. Recheck hidden-idle CPU, wake counts, and protocol traffic in the actual application after fixing this.

### F2. P1: A delayed invalidation can overwrite a different WebView session's view

Evidence: [ViewInvalidationDelivery.rs:95](../../apps/desktop/shell/src/ViewInvalidationDelivery.rs#L95), [SearchSession.rs:27](../../apps/desktop/shell/src/SearchSession.rs#L27), and [NavigationState.rs:88](../../apps/desktop/shell/src/NavigationState.rs#L88).

The invalidation worker captures a route but not its owning session ID. After waiting for the extension, it matches only `route_id` and `revision`. A new WebView session starts a new navigation state and can reuse both numbers. If extension A's old response arrives after the new session opens extension B's first route, A's view document can be written into B's route while B's ownership metadata remains unchanged.

The controlled probe reproduced exactly that replacement: session 2 retained extension B's identity but contained extension A's delayed content.

Correction: Carry and verify the originating session ID together with extension, view, route, and revision identity before applying a completion. Keep session replacement independent from old work completion. Add a deterministic stale-completion test that replaces the session while the extension response is pending.

### F3. P1: Quit relies on destructors that the Tauri run path does not guarantee

Evidence: [tray.rs:21](../../apps/desktop/shell/src/tray.rs#L21), [lib.rs:200](../../apps/desktop/shell/src/lib.rs#L200), [DesktopState.rs:505](../../apps/desktop/shell/src/DesktopState.rs#L505), and [RuntimeService.rs:362](../../engine/runtime/src/RuntimeService.rs#L362).

Quit directly calls `app.exit(0)`. The shell uses `Builder::run` without an application exit coordinator, while extension shutdown and storage/search cleanup are primarily in `Drop`. Tauri 2.11.5 documents that `App::run` exits through `std::process::exit`; Rust does not run stack destructors for that exit. Inspection of the installed Tauri source also shows that its resource-table cleanup is not an invocation of Nanika's runtime shutdown.

This leaves accepted-work settlement and owned-process termination outside a guaranteed application shutdown path. Windows Job Objects offer an OS lifetime backstop. The macOS process group requires explicit termination, so an extension or descendant that does not exit on pipe closure can survive. That macOS consequence is source-derived and was not physically reproduced here.

There is a second ordering problem within the fallback destructor: it joins view invalidation delivery before the runtime requests extension shutdown. A delivery worker waiting for a never-completing extension response cannot finish merely because a Shutdown wake is queued.

Correction: Add an explicit, idempotent application shutdown phase. Stop admitting new work, signal the existing extension shutdown policy, settle or close dependent completion paths, join delivery and service workers in dependency order, then allow the application to exit. Do not introduce an implicit timeout or retry. Verify pending initialization, a stalled view response, active host services, and process descendants on both platforms.

Sources: [Tauri App::run](https://docs.rs/tauri/2.11.5/tauri/struct.App.html#method.run), [Rust process::exit](https://doc.rust-lang.org/std/process/fn.exit.html).

### F4. P1: Rapid navigation can execute an action for the previous item

Evidence: [ExtensionView.svelte:198](../../apps/desktop/frontend/src/components/ExtensionView.svelte#L198), [ExtensionView.svelte:221](../../apps/desktop/frontend/src/components/ExtensionView.svelte#L221), and [App.svelte:324](../../apps/desktop/frontend/src/App.svelte#L324).

Selection changes are sent as nonblocking events, but the next keyboard event computes its position and action from the last host snapshot. Until the host round trip finishes, two Down presses both request the second item, and Enter still identifies the first item. The shell serializes requests but deliberately accepts older view revisions when the referenced item/action is still present; this does not repair the frontend's wrong target.

The extracted-handler probe, starting on item one and withholding a host update, produced `selectionChanged(two)`, `selectionChanged(two)`, and `actionInvoked(one)` for Down, Down, Enter. This is an input-intent bug, not merely an unmeasured frame-rate concern.

Correction: Keep an explicit local desired selection, as the frontend already does for query text, and reconcile it with authoritative view revisions. Make action submission refer to the intended selected item and define sequencing while selection is pending. Preserve every side-effecting action; only superseded selection intent may be coalesced under an explicit policy. Validate keyboard repeat, rapid Down/Enter, pointer selection, and selection disappearing after filtering in the real UI.

### F5. P2: Navigation-only changes retransmit the entire search catalog

Evidence: [SearchDelivery.rs:67](../../apps/desktop/shell/src/SearchDelivery.rs#L67), [RootSearchSnapshot.rs:8](../../apps/desktop/shell/src/RootSearchSnapshot.rs#L8), and [NavigationState.rs:17](../../apps/desktop/shell/src/NavigationState.rs#L17).

Search and navigation are embedded in one full snapshot. Changing only navigation busy state or an extension detail invalidation reconstructs every search result and includes the active view again. The transformation and channel send occur while holding the shared desktop-state mutex. A previously completed 2,000-result search therefore contributes all 2,000 rows to an unrelated navigation update.

The controlled delivery probe confirmed exactly that 2,000-row payload for a busy-only change. No production serialization latency was measured.

Correction: Preserve a single authorized channel if sufficient, but use independently versioned search/navigation/view sections or typed update variants so unchanged sections are omitted. A separate channel per feature is not required. Move expensive payload construction/serialization outside the shared lock while preserving delivery order, single in-flight acknowledgement, and stale-session checks. Compare real payload bytes and UI latency before and after.

### F6. P2: Successful action recording depends on presentation succeeding

Evidence: [DesktopState.rs:253](../../apps/desktop/shell/src/DesktopState.rs#L253), [DesktopState.rs:378](../../apps/desktop/shell/src/DesktopState.rs#L378), and [RuntimeService.rs:234](../../engine/runtime/src/RuntimeService.rs#L234).

`run_invocation` receives a successful extension result, applies its navigation effect, and only then calls `record_execution`. If the originating WebView session has closed or been replaced, navigation returns an error first. The action may already have launched an application or copied data, yet usage/history is not recorded and the orchestration path reports failure. A future non-desktop caller must also remember to perform a separate recording step.

Correction: Let the runtime own successful execution accounting independently from delivery to a window. At minimum, settle recording before optional presentation and keep execution, persistence, and presentation outcomes distinct. This needs no general event-sourcing framework. Add a test completing an action after its UI session closes and verify exactly one persisted successful execution.

### F7. P2: A global view-operation lock survives across remote waits and sessions

Evidence: [DesktopState.rs:267](../../apps/desktop/shell/src/DesktopState.rs#L267) and [ViewInvalidationDelivery.rs:42](../../apps/desktop/shell/src/ViewInvalidationDelivery.rs#L42).

Both user view operations and background invalidation hold one DesktopState-wide mutex while waiting for an extension completion. Replacing the WebView session does not replace that mutex. A stalled old extension request can therefore prevent interaction with a different extension's new route, even after the new session has opened it. The stale-result probe also verified that the operation mutex remains locked while the old response is pending.

Correction: Serialize operations at their actual owner, such as an extension/view or session-scoped operation owner, and correlate completions explicitly. Keep desktop state critical sections short. Review the existing per-extension worker serialization before introducing another queue. A new session must not queue behind a detached old session's unrelated view work. Preserve accepted operations and concrete failures instead of adding a watchdog.

### F8. P2: Nested view retention has no enforced depth boundary

Evidence: [NavigationState.rs:77](../../apps/desktop/shell/src/NavigationState.rs#L77) and [NavigationState.rs:89](../../apps/desktop/shell/src/NavigationState.rs#L89). The baseline [technical stack](../plan/tech-stack.md) states that nested routes are bounded.

Push checks only for a duplicate active view ID. Every distinct ID retains another complete view in the stack, with no depth or aggregate-memory admission boundary. Individual protocol-frame and view-field limits do not bound the number of retained views. The navigation probe accepted 1,000 distinct nested routes; this is an algorithm-level retention observation, not an app memory benchmark.

Correction: Define and enforce a documented navigation-depth admission limit. Reject a push explicitly at the boundary and close/release any newly created remote view that cannot be retained. Do not silently discard an older route. Add boundary and rejected-view cleanup tests; decide whether aggregate retained-view bytes also need a measured product limit.

## Structural optimization to schedule after correctness fixes

### Separate installed contributions from active processes

Evidence: [RuntimeService.rs:119](../../engine/runtime/src/RuntimeService.rs#L119), [ExtensionSearchWorker.rs:95](../../engine/runtime/src/ExtensionSearchWorker.rs#L95), and [ExtensionSearchWorker.rs:550](../../engine/runtime/src/ExtensionSearchWorker.rs#L550).

Every enabled extension is spawned during runtime startup. Even a static command/view contribution is published through a worker only after its process initializes; `run_query` also checks that the process is alive before publishing those static contributions. Consequently, the number of installed enabled extensions drives startup and resident process/thread cost, even when the user never invokes them.

This is the most relevant structural improvement from the research for a growing extension ecosystem. Keep a validated contribution catalog independent from process activation. Retain eager activation where current behavior requires it, such as application discovery or clipboard observation. Define on-demand activation for static command/view providers and show explicit unavailable state when activation fails. Keep built-in and external policy equivalent. Measure process counts, startup, first activation, and full-tree memory before changing defaults. Do not add speculative generic activation events, automatic eviction, or restart policy.

## Decisions that should remain unchanged

- Keep Rust-owned business/runtime state, the Tauri shell boundary, the typed frontend bridge, and shared declarative rendering.
- Keep Svelte 5 and native frontend state. This review found no framework limitation requiring Solid, Effect, a global state framework, or a component library.
- Preserve backpressure, concrete failure outcomes, and explicit cancellation. None of these findings calls for silent retries, response dropping, arbitrary timeouts, or transcript truncation.
- Do not implement ACP sessions, agent contributions, chat persistence, multi-endpoint packages, or MCP integration as part of these fixes.
- Do not treat semantic host-owned icons or a currently small view vocabulary as defects merely because future features may add more variants.
- Do not change the initial coherent-search barrier as a side effect of this review; it is an explicit current product policy. Revisit its interaction with slow workers only through a separate behavior decision.

## Suggested order

1. Remove the preparation feedback loop and measure hidden idle.
2. Fix stale session completion checks and rapid selection/action targeting.
3. Establish explicit shutdown and correct operation-owner lifetime boundaries.
4. Separate execution recording from presentation.
5. Avoid retransmitting unchanged snapshot sections and enforce navigation admission bounds.
6. Measure and design on-demand activation for eligible extensions.

Use focused regression tests for the confirmed races and protocol boundaries, then actual Windows/macOS application acceptance for latency, keyboard behavior, shutdown, and resource use. Passing the existing repository suite did not cover the reproduced cases above.
