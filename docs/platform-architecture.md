# Platform Architecture

This document records ownership and platform contracts. Code and manifests define
implemented behavior; [tasks](tasks.md) holds unfinished candidates and validation
gaps. Supported targets are Windows 10+ and macOS 13+ only.

## Responsibility boundaries

| Location              | Responsibility                                                                                       |
| --------------------- | ---------------------------------------------------------------------------------------------------- |
| engine/foundation     | Product identity, extension IDs and diagnostic primitives                                            |
| engine/platform       | Native mechanisms: containment, file replacement, launch/reveal, target selection and OS integration |
| Other engine crates   | Search, protocol, supervision, configuration, storage and package policy, independent of Tauri       |
| apps/extensions       | Domain capabilities and their platform-specific discovery or capture adapters                        |
| apps/desktop/shell    | Tauri commands, channels, windows, capabilities, Isolation and WebView sessions                      |
| apps/desktop/frontend | One Svelte renderer for host and declarative extension surfaces                                      |
| tooling               | Development, build and validation, never an installed-app dependency                                 |

Adapters may change mechanisms, not shared product, transaction, cancellation or
failure semantics. Keep native APIs, handles and path resolution at their owning
adapter boundary. Unsupported platforms fail explicitly. Before changing a
platform contract, identify ownership and both implementations; report actual
runtime evidence separately from compilation.

Extensions are the only domain capability unit. Built-in provenance grants no
runtime exception. See [extension lifecycle](extension-lifecycle.md) for current
registration and the deferred live-lifecycle proposal.

## IPC and execution authority

Frontend access goes through the typed bridge and explicit shell permissions.
Rust validates bounded requests against the WebView session, route, current action
metadata and extension permissions. Extensions supply data and declarative nodes,
never frontend code, raw DOM access or arbitrary filesystem/process access to UI.

Search submissions use invoke; the response acknowledges submission. Result and
navigation state arrive on the launcher's long-lived session Channel. Settings has
its own session Channel. Isolation validation and bounded delivery apply to both;
a queued send is not acknowledged receipt.

Root execution binds to the immutable delivered search snapshot. Its replacement
invalidates the reviewed target even within the same query generation. Menus and
confirmation bind to exact reviewed revisions; ordinary queued view input retains
stable IDs and is revalidated against the current route under the shared
operation/invalidation lock. Never predict revisions or retarget stale actions.

The frontend view-input FIFO coalesces only adjacent unsent selections for the same
session/route. Actions are barriers. Overflow, replaced routes and unavailable
targets fail explicitly. Blocking input waits for RPC completion and the correlated
Channel revision in either order; selection stays nonblocking. Preserve accepted
work, concrete failure causes and backpressure rather than adding silent retries,
timeouts, recovery or whole-app restarts.

See [design system](design-system.md) for activation and confirmation UX.

## Settings operations

Settings submits one property key and value. Rust validates visibility and schema,
merges the value with the authoritative configuration, and reserves one
operation per extension until all stages settle. This is the extension's atomic
configuration domain; unrelated extensions remain independent. Frontend state
admits one edit per field and serializes distinct fields within that domain,
without disabling or replacing the page. Dropping a receipt or hiding Settings
does not cancel accepted work. Shutdown interrupts extension work and waits for
configuration transactions to settle. Field metadata uses reactive maps, so all
schema-valid keys remain addressable. Failed validation retains the user's draft
without retrying on navigation. Commit preparation validates the complete property
and converts its draft to schema-defined values before no-op detection or admission.
The writer receives that same prepared value. No-op detection compares durable and
authoritative presentation values and retains unresolved application failures.

Every configuration property explicitly declares `persistence`:

- `beforeApply`: persist desired configuration, then apply it. Application failure
  retains the durable intent. An inactive extension receives it on activation.
- `afterApply`: require live application confirmation, then persist. A failed or
  unavailable extension cannot save the requested value. If application succeeds
  and persistence fails, report effective new values and saved old values without
  inventing rollback. An on-demand extension is activated to obtain confirmation.

The completed result contains `values` for presentation, `saved`, nullable
`effective`, and a concrete error. `effective: null` means unconfirmed, not reverted.
`ConfigurationApplied` confirms the entire requested snapshot after domain work
finishes; submission, queueing, progress and dormant initialization data are not
application confirmation. Host preferences and native startup adapters use the
same frontend lifecycle. Windows startup remains owned by its native startup
service and macOS by its native login-item service; OS state is queried after a
failed operation rather than persisting an assumed toggle value. Returning to the
active Settings window or General page reads native startup state again. Reads
and writes share the same serial domain; stale reads never replace queued edits.

`ConfigurationProgress` carries the request ID, a bounded label and completed/total
work units. A null total means indeterminate. The host validates progress and routes
it to the same operation on the Settings Channel; terminal state cannot regress to
progress. The shell permits one unacknowledged progress delivery per Settings
Channel and retains only the latest pending progress per installed extension,
serving extensions in arrival order. `acknowledge_settings_progress` releases that
slot only for its delivery ID; IDs are not reused when the WebView subscribes again.
Terminal results bypass progress backpressure and remove obsolete pending progress.
Channel sends happen outside the shared state lock. There is no timer or retry in
this delivery contract. Reporting 100 percent is not completion. Application discovery reports
source work and final index reconciliation. Other extensions can use the same
protocol without supplying frontend code. Settings uses a one-shot presentation
delay, no progress polling, fake percentages, automatic retries or operation timeout.

Requested, durable and effective state are separate, consistent with the
[Windows Radio completion contract](https://learn.microsoft.com/en-us/uwp/api/windows.devices.radios.radio.setstateasync)
and [GTK requested/backend state](https://docs.gtk.org/gtk4/class.Switch.html).
These references guide state semantics, not dependencies or native animation.

## Build and development

Use the root package, pinned Bun and one bun.lock. Bun is tooling only; installed
releases must need neither Bun nor Node. Derive sidecar build/staging from Tauri
bundle.externalBin. Inspect final artifacts after packaging/toolchain changes.

Dev, build, check and Computer Use share an exclusive build lock through compile,
stage, package and launch. Tooling tests use a separate lock. Failed builds must
not publish stale bundles. Development refuses a running Nanika instance and owns
its Vite listener; an occupied port is an error, not permission to reuse a server.

macOS tooling cancels its process group and waits for exit. Windows uses hidden
taskkill /T /F and waits for the child. Report termination failures honestly;
tooling cancellation does not define extension runtime shutdown policy.

Temporary output belongs under target; frontend output under
apps/desktop/frontend/dist. just clean explicitly removes target after builds and
the development app stop. Startup does not perform automatic cache eviction.

## Application discovery and reconciliation

The application extension owns source constants and configuration. All built-in
sources default to enabled. Settings displays current-platform built-in switches
before custom folders, retaining other-platform configuration values. Schema
visibility and ordering apply equally to either extension origin; presentation
does not replace validation. Saves require current-platform fields; omitted hidden
fields are preserved, while supplied hidden values are validated.

| Platform | Built-in source resolution                                                                                                           |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Windows  | Known Folder IDs for Start Menu and packaged-app roots; SCOOP/SCOOP_GLOBAL or Known Folder profile/ProgramData roots for Scoop shims |
| macOS    | /Applications, /System/Applications and the current HOME/Applications                                                                |

Do not embed a username or installation drive. Invalid overrides and resolution
errors remain concrete failures instead of falling back silently. Scoop discovery
visits shims, not versioned apps or backup/cache directories.

Windows shortcut/executable identity uses the canonical target and arguments.
Wrappers with paired .shim metadata resolve that identity while activation uses
the original wrapper or shortcut. Bound metadata reads, report malformed content
and omit missing targets.
Wrappers with additional environment, working-directory, elevation or variable
semantics remain distinct. Distinct targets or argument variants must not merge
merely because their display names match. macOS bundle identity uses its bundle ID,
falling back to the executable path.

Configuration application triggers a transactional rescan. An uninterrupted scan
replaces records in successful paths and removes missing paths or disabled/removed
roots, while preserving records under failed files or subtrees. Entries found
through other enabled sources remain eligible. A cancelled scan performs no
deletion. Unknown root resolution limits cleanup to known roots; a failure with a
resolved path protects only that path. Matching respects directory boundaries.
Discovery failures are logged and scan warnings recorded while other sources
continue; failure must not turn an unscanned source into an empty result.

The source and reconciliation contract is implemented in the application
extension's [platform adapters](../apps/extensions/built-in/application/src/platform.rs)
and [index](../apps/extensions/built-in/application/src/ApplicationIndex.rs).

## Native presentation and reveal

Windows default-index application icons share native extraction and transparent
bounds normalization with clipboard file icons. Explicit resource indices use
separate extraction. Revise Windows cache identity when rendering changes; macOS
retains its own extraction and cache policy.

Tauri Focused(false) hides the launcher when hide-on-blur is enabled. DOM activity
controls visual work and menu dismissal only. In-WebView context menus create no
native window and do not require a second launcher focus manager.

The shell presents Settings after its retained WebView is ready. Closing hides the
native window and resets navigation while retaining field edits and operations. There is no custom
opening/closing transition or animation acknowledgement. Windows uses undecorated,
transparent surfaces and custom Settings controls; native shadows are disabled so
CSS corners stay transparent. macOS Settings retains the native titlebar, traffic
lights and gestures. Maximized content removes CSS rounding. Tauri configuration
differences belong in shell adapters, separate from engine-native mechanisms.

File reveal is a permission-checked host service executed off the UI thread.
Windows uses Shell item selection with DOS/UNC spelling at the Shell boundary;
canonical filesystem identity stays unchanged. macOS asks NSWorkspace to select
file URLs in Finder. Report observable errors; successful request submission is
not proof of the file manager's final rendered state.

## Validation boundaries

Browser fixtures and cross-compilation do not establish native correctness. Report
the actual platform and scenarios tested. Outstanding platform, packaging and
measurement gaps are tracked in [tasks](tasks.md).
