# Extension Lifecycle

Built-in and external extensions share one runtime contract. The installation and
configuration foundation is implemented; live enable/disable remains a proposal.
Its implementation is deferred until the foundation is committed and separately
authorized. Offline enablement changes take effect at the next host startup.

## Current ownership and behavior

Domain capabilities run in extension processes. Rust owns supervision, search,
configuration, storage and permission-checked host services. The desktop shell
owns windows, sessions and the shared declarative renderer. Built-in identity grants no domain-specific host dispatch or frontend branch.

Host-owned inventory establishes built-in provenance. External package validation
rejects reserved IDs. Both feed the same InstalledExtension descriptor, without an
origin flag, and the same RuntimeService registration path. Inventory parsing
is not signature verification; release validation is tracked in [tasks](tasks.md).

The host database's extensions table records installation identity and package
provenance, not enablement. extensions.jsonc is the sole persisted enablement
source, enabled by default for either origin. Offline management accepts both.
Incompatible pre-release schemas are rejected without migration or automatic
removal of user data.

Validated, resolved descriptors enter the configuration registry and Settings
metadata before the shared startup enablement gate. Disabled extensions create no
worker, but retain editable configuration. Extensions without a configuration
schema remain listed; configuration failures remain visible with their cause.
Missing executables or invalid packages produce diagnostics and are not resolved
into Settings entries. Installed metadata is not a claim of runtime readiness.

The worker collection is fixed after startup. Settings has no extension enable
Switch or live start/stop API. Saving an enabled extension's configuration is a
separate supported operation; persistence and runtime application have distinct
outcomes. `beforeApply` values can be saved while disabled for the next enabled
startup; `afterApply` requires live confirmation and cannot save while disabled.
See the [Settings operation contract](platform-architecture.md#settings-operations).

Source: [runtime registration](../engine/runtime/src/RuntimeService.rs),
[enablement authority](../engine/configuration/src/ExtensionRegistryConfig.rs),
[offline management](../engine/extension-management/src/package.rs) and
[contract tests](../engine/runtime/tests/RuntimeService.rs).

## Proposed live lifecycle

Reuse engine/runtime for lifecycle
policy, engine/extension-management for packages, engine/platform for process
mechanisms and the shell for sessions. Do not add a second scheduler, special
built-in path or invisible whole-app restart.

Maintain the installed descriptor separately from an optional live instance.
Assign each new instance a host-owned monotonic identity. Search publications,
view routes, invalidations, configuration completions and host-service requests
must retain that identity. Query generations alone cannot distinguish process
lifetimes. Retired instances must never authorize or publish into a replacement.

Persist enabled as desired state; report actual startup, readiness, stopping and
failure separately. An enabled on-demand extension may have no process. Saving a
choice does not prove transition completion. Preserve the choice and concrete
failure without automatic retries.

Serialize transitions per extension with bounded admission. Reject conflicting
transitions explicitly; never discard accepted work. Unrelated extensions remain
usable. Application shutdown closes lifecycle admission before waiting. Do not
hold catalog or UI locks while awaiting a process.

### Disable

1. Persist the choice; on failure retain the previous live state and report why.
2. Close admission, withdraw contributions and retire view routes for the instance.
3. Resolve accepted requests with truthful outcomes, preserving completion of
   already submitted host services that may have irreversible effects.
4. Stop producers and drain durable writes under the extension's cleanup contract.
5. Send Shutdown and await its acknowledgement and clean process exit.
6. Release instance resources and publish disabled only after stopping succeeds.

Superseded searches may be cancelled. Pending actions that have not started need
an explicit cancellation result. Do not claim cancellation of an effect that may
already have executed. Keep configuration and diagnostics accessible throughout.

Current application termination uses ExtensionSearchWorker::request_stop and
process termination. It is not a graceful user-disable implementation. Although
Shutdown/ShutdownAck already exist, Clipboard acknowledges before stopping its
monitor and draining its worker. Acknowledgement alone cannot prove persistence
or process exit. The control path must remain responsive during active work.

If stopping remains pending or fails, preserve that actual state and cause. Do not
invent a timeout, forced kill or restart policy. An uncooperative native process
cannot guarantee both instantaneous exit and lossless cleanup. A future explicit
force-stop operation needs its own data-loss and failure contract.

### Enable

1. Persist the choice for a validated installed descriptor.
2. Allocate a fresh instance identity with the latest configuration and permissions.
3. Apply the manifest activation rule: start immediately or register on-demand work.
4. Publish availability after registration and send ready contributors the current query.

### Settings and packages

Use the shared Switch for every resolved installed extension, including those
without configuration properties. Save directly and expose pending/failure state
through the Settings channel. Retain settings and user data while disabled.

Live install/uninstall is a separate candidate. Installation would publish an
inventory delta after validation commits; uninstall would complete disable before
removing registration and files. Built-in bundle files remain release-owned;
package ownership grants no runtime privilege.

## Acceptance requirements for the proposal

- Run the same lifecycle cases for both origins: startup, on-demand, disabled,
  configuration-free, failed initialization and zero-extension host.
- Cover disabling during initialization, queries, actions, configuration updates,
  host services and persistence; accepted work receives one truthful outcome.
- Re-enable with late messages from the old instance and reused request/view IDs.
- Cover rapid toggles, host shutdown, persistence failure, full queues and an
  uncooperative process without silent recovery or focus changes.
- Verify native process exit and capability withdrawal on Windows and macOS.
- Measure withdrawal, exit and reactivation separately from unrelated UI latency.

Foundation tests recreate the host; they do not prove live toggling.
