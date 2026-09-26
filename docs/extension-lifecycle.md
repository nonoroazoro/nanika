# Extension Lifecycle

Built-in and external extensions share one live enable/disable contract. Rust owns
lifecycle/configuration transactions; the shell owns sessions and route retirement.
See [architecture](platform-architecture.md) for storage, scanning and catalog delivery.

## Installation and runtime identity

Host inventory alone establishes built-in provenance; external packages cannot claim
reserved IDs. Both origins use the same installed descriptor, permission/configuration
registry and RuntimeService path. Inventory parsing is not signature verification.

The host DB records installation/provenance. `extensions.jsonc` is the sole durable
enablement authority, enabled by default. All writers, including offline package
management, hold the OS lock on `extensions.lock` while reading, changing or rolling
back the registry. This lock is separate from the atomically replaced document and
prevents rollback from overwriting another writer's committed choice.

Installed descriptors outlive instances. Disabled and configuration-free extensions
remain listed. Invalid configuration retains its concrete error; explicit enable
re-reads it so manual corrections can take effect. Invalid packages/missing executables
remain resolution diagnostics, not runnable Settings entries.

Each instance receives a host-owned monotonic identity independent of query and
extension-owned IDs. Its publication gate scopes search, host-service admission and
view updates. Retirement withdraws contributions through an acknowledged search-owner
barrier shared with query admission. Late replies cannot mutate replacement routes or
permissions. Accepted action output remains coordinator-owned with its originating
identity. Removing a retired route preserves unrelated live routes/resources.

## Desired state, admission and actual state

`enabled` is durable intent; `pending` is an accepted lifecycle operation. Actual state
is `disabled`, `dormant`, `starting`, `ready`, `stopping` or `failed`. The worker owns
process state; operation diagnostics do not redefine readiness. An on-demand extension
may be dormant with registered static entries and no process.

Startup and explicit enable use the same worker-owned factory and latest accepted
initialization configuration. Transport exit wakes even an idle worker. Worker exit
without a terminal stop result, including panic, is failure.

Configuration and lifecycle share one reservation per extension through application,
persistence, reconciliation and completion. Conflicts fail explicitly; unrelated
extensions remain usable. Hiding Settings/dropping receipts does not cancel accepted
work. Shutdown closes admission before settling operations. Catalog/UI locks are not
held while waiting for processes.

## Launcher catalog refresh

Native opens coalesce into one queued/active `Refresh` / `Refreshed` exchange for
dynamic contributors. Refresh completion acknowledges discovery; changed catalogs
publish `CandidatesChanged`. Replies dispatch independently of queries/actions, while
mutations serialize with configuration. Invocations retain worker priority.

Catalog transactions belong to the instance, not the query. Staged batches are invisible
until search-owner commit and `CatalogApplied`; retirement revokes publication authority.
Query contributors retain generation-scoped responses. Neither mode changes navigation.
See [publication and search](platform-architecture.md#catalog-publication-and-search).

Built-in scanners publish each completed recursive root. Exit discards incomplete
staging; restart scans from the beginning without checkpoints. One extension's refresh
failure does not undo another's catalog. Shutdown interrupts waits before joining owners.

## Disable

1. Persist the choice. Persistence failure leaves the live instance admitted.
2. Close admission and withdraw that instance's contributions/routes.
3. Settle accepted work, explicitly cancelling queued actions not yet started and
   retaining real outcomes for active actions/submitted host services.
4. Close stdin after active work settles. Drain terminal responses for cancelled searches.
5. Await cleanup, process/descendant exit and output draining.
6. Publish disabled and release the instance only after successful cleanup.

Extensions stop producers before draining durable writes. Clipboard stops its monitor,
joins its icon worker and drains its DB owner; cleanup failures reach the host through
nonzero exit and retained stderr. Pending/failed cleanup stays visible and retains
ownership, preventing replacement. Explicit disable has no timeout, forced kill or
automatic retry. Application shutdown retains its separate termination policy and can
interrupt a pending graceful stop.

## Enable

1. Persist the choice for a resolved installed descriptor.
2. Allocate an identity with current saved configuration and manifest permissions.
3. Start startup-activated extensions or register on-demand work.
4. Feed registered contributors the current query.
5. Publish readiness independently of saved intent.

Restart uses saved values, not unpersisted effective values from a retired instance.
`beforeApply` can save while disabled; `afterApply` cannot. See
[Settings operations](platform-architecture.md#settings-operations).

## Unexpected failure and recovery

Capture, copy, query and configuration failures affect their operation while transport
is healthy; they do not implicitly disable the extension or invalidate later cleanup.

RuntimeService's event-driven supervisor allows one automatic restart per explicit
enable cycle after transport exit or failed activation, including idle disconnect.
Recovery shares configuration/lifecycle admission, preserves durable intent, withdraws
the old instance and terminates remaining containment before replacement. It initializes
saved configuration/current query and immediately starts crashed on-demand extensions.
Accepted actions/view requests retain terminal failures and are never replayed.

A second failure stops recovery; Settings retains both causes and permits explicit
disable/enable. Cleanup failure also blocks replacement. Shutdown closes recovery
admission and interrupts waits. There is no polling, restart timer or persisted counter.

## Protocol and native process contract

Pre-1.0 redesigns replace the current protocol directly, preserving
`protocolVersion: 1` and `nanika.extension.v1`. No compatibility branches or migrations.

EOF is the process lifetime boundary for Nanika and ACP. Nanika has no shutdown
acknowledgement that substitutes for completed writes/descendant exit. ACP uses v1
JSON-RPC sessions. Neither transport adds an application-defined message byte quota.

ACP `session/cancel` cancels a prompt, not a process: await its terminal response before
retiring transport. Dropping the connection closes stdin, drains stderr and waits for
exit. Do not synthesize success or add a private shutdown request. References:
[stdio transport](https://agentclientprotocol.com/protocol/transports) and
[cancellation](https://agentclientprotocol.com/protocol/cancellation).
SDK-specific termination deadlines are not product policy.

| Platform | Containment and graceful completion                                                                                                            |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows  | Start suspended, assign Job Object, resume; require zero active Job processes. Kill-on-close is an ownership safeguard, not graceful disable.  |
| macOS    | Establish process group before launch; require group absence via signal zero. Shutdown/recovery signal the group before releasing containment. |

Children own graceful descendant cleanup. Open pipes or surviving descendants keep
retirement pending. Native adapters expose observations, not lifecycle policy.

## Settings observations and validation

Every resolved extension has a separate first enablement group, including extensions
without configuration. Pending domain edits do not dim/disable enablement; the runtime
rejects an actual conflicting lifecycle request without changing accepted work.

The Settings Channel reports lifecycle/configuration even without a launcher session.
Lifecycle retains one unacknowledged snapshot and the latest state. Receipt IDs share
the progress sequence and never repeat across subscriptions. Configuration facts and
save results use their own runtime revision, independent of lifecycle or schema version.
Stale facts cannot overwrite newer saves; operation errors still arrive. Corrected
configuration creates edit state before exposing fields. Accepted edits settle before
lifecycle reconciliation, while uncommitted drafts remain user-owned. Route retirement
does not hide the launcher or move native focus.

Tests cover both origins, zero-extension startup, registry/rollback isolation, shared
admission, persistence/cleanup failure, instance replacement, unrelated queries, idle
exit, worker panic and slow Settings consumers. ACP tests use real child processes.
Native platform/release gaps remain in [TODO](tasks.md); live install/uninstall is not
part of this implemented contract.

Source: [runtime](../engine/runtime/src/RuntimeService.rs),
[registry transactions](../engine/configuration/src/ExtensionRegistryTransaction.rs),
[protocol](../engine/extension-protocol/src/Message.rs),
[lifecycle tests](../engine/runtime/tests/RuntimeService.rs).
