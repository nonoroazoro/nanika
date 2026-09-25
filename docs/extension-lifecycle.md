# Extension Lifecycle

Built-in and external extensions share one live enable/disable contract. Settings
changes desired enablement without restarting the host. Rust owns lifecycle and
configuration transactions; the desktop shell owns sessions and route retirement.

## Installation and runtime identity

Host inventory alone establishes built-in provenance. External packages cannot
claim reserved IDs. Both resolve to the same InstalledExtension descriptor,
permissions, configuration registry and RuntimeService registration path. Inventory
parsing is not signature verification; release validation remains in [tasks](tasks.md).

The host database records installation identity and package provenance.
`extensions.jsonc` is the sole durable enablement authority, enabled by default.
Every writer, including offline package management, acquires the same OS file lock
on `extensions.lock` before reading, changing or rolling back the registry. The
lock is separate from the atomically replaced document. Package rollback retains
this lock so it cannot restore a snapshot over another writer's committed choice.
There are no pre-release compatibility paths, migrations or automatic data resets.

Installed descriptors outlive runtime instances. Disabled and configuration-free
extensions remain listed. Configuration errors retain their concrete causes. Explicit enable re-reads an
initially invalid configuration so a user correction can take effect.
Invalid packages and missing executables remain resolution diagnostics rather than
claiming a runnable Settings entry. Each new instance has a host-owned monotonic
identity, independent of query generations and extension-owned request/view IDs.

An instance publication gate binds search, host-service admission and view updates
to that lifetime. Retirement withdraws search contributions through an acknowledged
search-owner barrier. Query admission shares the withdrawal barrier so a new query
cannot wait for an already retired contributor. Routes and invalidations retain the
instance identity; late replies cannot mutate replacement routes or permissions.
Accepted action output remains coordinator-owned after a worker stops and carries
its originating instance identity. Removing a retired route preserves other live
extensions' routes and resources.

## Desired state, admission and actual state

`enabled` records durable intent. `pending` records an accepted lifecycle operation.
Actual state is `disabled`, `dormant`, `starting`, `ready`, `stopping` or `failed`.
The worker owns process state; request diagnostics do not redefine readiness.
Startup and explicit enable use the same factory, owned by the worker. The factory
receives the latest accepted initialization configuration when activation occurs.
Transport termination notifies the worker even while idle. A worker that exits
without a terminal stop result reports failure, including unwinding after a panic.
An enabled on-demand extension can be dormant with a registered static catalog and
no process. Saving intent is not proof of readiness or successful cleanup.

Configuration and lifecycle share one reservation per extension. Conflicting
requests fail explicitly. The reservation spans domain application, any required
persistence, outcome reconciliation and transition completion. Other extensions
remain usable. Neither hiding Settings nor dropping a receipt cancels accepted work.
Application shutdown closes admission before settling operations. Catalog and UI
locks are not held while waiting for process completion.

### Disable

1. Persist the choice. A persistence failure leaves the live instance admitted.
2. Close admission and withdraw contributions and routes for the retiring instance.
3. Settle accepted work. Cancel queued actions that have not started explicitly;
   retain actual terminal outcomes of active actions and already submitted host services.
4. Close process stdin after active work settles. Superseded searches may use their
   protocol cancellation, whose terminal response must still be drained.
5. Await extension cleanup, process exit, descendant exit and output draining.
6. Publish disabled only after successful cleanup and release the instance.

Extensions stop producers before draining durable writes. Clipboard stops its
monitor, joins its icon worker and drains its database owner; cleanup failures
propagate through a nonzero exit and retained stderr diagnostics.

Pending or failed cleanup remains visible. Explicit disable has no timeout,
forced kill or automatic retry. A failed stop retains ownership and prevents a
replacement instance. Explicit application shutdown retains its existing process
termination policy and can interrupt an indefinitely pending graceful stop.

### Enable

1. Persist the choice for a resolved installed descriptor.
2. Allocate a new identity with current durable configuration and manifest permissions.
3. Start startup-activated extensions or register on-demand work.
4. Feed the current query to registered contributors and publish readiness separately.

Restart initialization uses saved values, never unpersisted effective values from
a retired instance. `beforeApply` configuration may be saved while disabled;
`afterApply` requires live confirmation and cannot save while disabled. See the
[Settings operation contract](platform-architecture.md#settings-operations).

### Unexpected failure and recovery

A failed capture, copy, query or configuration operation reports only that operation's
failure. It does not disable the extension, terminate its process or turn a later
successful cleanup into a failed stop. Later work remains admitted while the
transport is healthy.

RuntimeService owns an event-driven supervisor independent of desktop visibility.
An unexpected transport exit or failed activation gets one automatic restart per
explicit enable cycle. This includes an idle disconnect. Recovery shares admission
with configuration and explicit lifecycle operations, preserves durable enablement,
withdraws the old instance and terminates its remaining containment before creating
a new instance. It initializes with saved configuration and the current query;
on-demand extensions that crashed are started immediately. Accepted actions and
view requests retain terminal failures and are never automatically replayed.

A second failure ends automatic recovery. Settings retains both the original cause
and the restarted instance's failure, with an explicit disable/enable option to try
again. Cleanup failure also prevents replacement and remains visible. Explicit
application shutdown closes recovery admission and interrupts process waits. No
polling, restart timer, crash loop or persisted restart counter is used.

## Protocol and native process contract

Before Nanika 1.0, protocol redesigns directly replace the initial design while
keeping all Nanika-owned internal versions at their initial values, including
`protocolVersion: 1` and `nanika.extension.v1`. There is one current
contract, with no compatibility branches or migrations.

EOF is the process lifetime boundary for both Nanika and ACP. Nanika has no
Shutdown/ShutdownAck exchange: an acknowledgement cannot prove that producers,
writes or descendants have finished. ACP uses ACP v1 and its native JSON-RPC
session protocol.

For ACP, `session/cancel` cancels a prompt, not a process. The host still awaits the
prompt's terminal response before retiring its transport. Dropping the client
connection closes stdin; the adapter drains stderr and waits for exit status.
ACP does not gain a private Nanika shutdown request or a synthesized success reply.

Native adapters expose containment observation without introducing lifecycle policy:

- Windows starts the child suspended, assigns its Job Object, then resumes it.
  Graceful completion requires zero active Job processes. Kill-on-close containment
  remains a final ownership safeguard, not the implementation of graceful disable.
- macOS creates the process group before launch. Graceful completion requires the
  group to be absent, tested with signal zero. Application termination and unexpected
  failure recovery use the group termination signal before releasing containment.

A child remains responsible for its descendants' graceful cleanup. Descendants that
hold pipes open or remain in the containment scope keep the transition pending.
The supported platforms remain Windows 10+ and macOS 13+.

The ACP contract follows the official
[stdio transport](https://agentclientprotocol.com/protocol/transports) and
[cancellation](https://agentclientprotocol.com/protocol/cancellation) rules.
SDK-specific automatic termination deadlines are not adopted as product policy.

## Settings and validation

Every resolved extension places Enable extension in its own first settings group,
separate from the extension's domain configuration. Both groups use the same
setting rows, Switch and pending feedback, with normal spacing between groups.
Configuration pending feedback belongs to the edited field and does not disable or
dim the enablement switch. The runtime admission gate rejects an actual overlapping
lifecycle request before changing enablement; Settings reports the error and keeps
the accepted configuration operation intact.
The enablement group remains available for extensions without domain configuration. A revisioned Settings channel reports lifecycle and configuration
observations even when no launcher session is open. Lifecycle delivery retains one
unacknowledged snapshot and the authoritative latest value. Receipt IDs are shared
with progress delivery and never reused across Settings subscriptions. Late snapshot replies cannot
replace newer observations. Configuration snapshots and save outcomes also share
a runtime-owned monotonic configuration revision, independent of lifecycle delivery
revision and persisted format versions. Older configuration facts cannot overwrite
newer save results; operation errors are still delivered. A corrected initially
invalid configuration creates its edit state before fields become visible.
Accepted edits settle before lifecycle reconciliation;
uncommitted drafts remain owned by the user. Retiring a route changes navigation
without hiding the launcher or moving native focus.

Tests cover live cycles for both origins and configuration shapes, zero-extension
startup, registry writer serialization, rollback isolation, shared configuration
admission, failed persistence, delayed/failed cleanup, new instance identities and
continued queries from unrelated extensions. ACP tests execute real processes and
check EOF cleanup, failure diagnostics and descendant completion. Additional
coverage separates operation failures from process failures, detects idle transport
exit and worker panic, and bounds delivery to slow Settings consumers. Native
acceptance includes current-query reactivation and retirement of an open route.
Windows execution does not establish macOS behavior; mixed-DPI, native focus/input,
minimum-platform and timing acceptance remain validation gaps in [tasks](tasks.md).

Live install/uninstall remains a separate candidate. Installation would publish an
inventory delta only after validation commits; uninstall would finish disable before
removing registration and files. Built-in bundle files remain release-owned.

Source: [runtime](../engine/runtime/src/RuntimeService.rs),
[registry transactions](../engine/configuration/src/ExtensionRegistryTransaction.rs),
[process protocol](../engine/extension-protocol/src/Message.rs), and
[lifecycle tests](../engine/runtime/tests/RuntimeService.rs).
