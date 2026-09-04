# Nanika Tasks

Status: Tauri is the only pre-1.0 desktop UI baseline. The existing Rust core remains the product foundation. Every unchecked item is a TODO.

## Product baseline

Nanika is a keyboard-driven capability host for Windows 10 and macOS 13 or later. Application search, commands, scripts, calculation, clipboard history, and future domain capabilities run as extensions.

Built-in and external extensions run as independent host-supervised processes with the same lifecycle, settings, permissions, diagnostics, and failure policy. Built-in status grants no additional privilege. A failed extension must not prevent the host or unaffected features from starting.

Extensions are Nanika's only first-class domain capability unit. The bare host owns infrastructure, orchestration, shared control-plane surfaces, and an empty Root Search surface, but contributes no candidates or domain actions. The shared frontend renders every in-app surface; the shell retains only unavoidable operating-system surfaces and pre-WebView recovery. No extension ships or injects frontend code.

Rust owns search, ranking, storage, configuration, diagnostics, extension supervision, host services, and platform integration. Tauri owns the desktop shell. A shared Svelte 5 and TypeScript frontend built with Vite and pnpm owns presentation and local interaction. Native behavior remains isolated behind typed Windows and macOS adapters.

The Tauri solution is the current design. Reusable UI-independent Rust code stays. UI-specific code outside the Tauri solution is deleted. There is no compatibility layer or parallel UI.

## Retain

Retain these implemented foundations and expose them to the Tauri shell through UI-independent Rust APIs:

- A configurable global hotkey toggles a focused overlay on the active monitor.
- Root Search waits for the initial snapshot from every ready extension before publishing a query generation and returns at most 100 ranked results.
- Application search uses localized macOS display names while retaining original names as aliases.
- Application icons are extracted, normalized by visible alpha bounds, and cached outside the Tauri event-loop and WebView main thread.
- Calculator contributes a result only when the query contains an explicit symbolic or word operator.
- The default distribution includes application search, commands, scripts, calculation, and clipboard history.
- Extensions contribute bounded incremental candidates, static commands, declarative views, and typed actions without host domain registration.
- Stale generations cannot replace newer results, and repeated execution cannot overwhelm lexical relevance.
- External extensions can be installed, repaired, updated, enabled, disabled, and removed through `nanika-cli` while Nanika is stopped.
- Configuration is relocatable, while databases, extension artifacts, payloads, caches, and logs remain machine-local.
- User-visible diagnostics remain separate from redacted operational diagnostics.
- Blocking storage, filesystem, image, and extension work stays outside the frontend and desktop event-loop threads.

Retain these source boundaries:

- `nanika-core`, `nanika-config`, `nanika-extension-package`, `nanika-platform`, `nanika-protocol`, `nanika-storage`, `nanika-search`, and `nanika-cli`.
- Extension protocol types, process supervision, lifecycle, restart, cancellation, deadlines, permissions, and host-service routing.
- Search ownership, ranking, query generations, input history, usage persistence, and snapshot publication.
- Application discovery, localized names, icon extraction, icon normalization, cache generation, and typed launch descriptors.
- Configuration, settings validation, atomic JSONC persistence, startup integration, diagnostics, and packaging logic that has no UI dependency.
- Active-monitor placement, overlay visibility, single-instance, startup, clipboard, and process-launch platform adapters. Keep `raw-window-handle` only while a retained adapter requires it. Retain native hotkey timing observation, but move registration to the official Tauri global-shortcut plugin.
- Native hotkey delivery timing, activation IDs, the slow-activation threshold, and redacted performance diagnostics. Replace renderer milestones with Tauri visibility, frontend readiness, and focus milestones.
- Every built-in and external extension process and its existing protocol boundary.

## Extension-first architecture

- [ ] Define and enforce the bare-host boundary: lifecycle, extension management, permission-checked generic host services, search aggregation and ranking, storage, diagnostics, configuration, platform adapters, Root Search composition, Settings composition, and shared presentation contracts. The bare host must start with zero extensions and show coherent empty and diagnostic states without contributing a domain candidate or action.
- [ ] Audit the frontend, shell, engine, storage schemas, and platform adapters for application, command, script, calculator, clipboard, agent, or other capability-specific branches. Move every such implementation behind an extension protocol boundary or delete it when obsolete.
- [ ] Preserve one extension path for built-in and external packages. Built-in status may affect default installation, enablement, update ownership, and uninstall policy only; it must not change protocol negotiation, permissions, process isolation, host services, view rendering, action routing, failure handling, or diagnostics.
- [ ] Make built-in identity host-owned distribution metadata. Bundle each built-in executable with a manifest using the ordinary extension schema, use reviewed version-controlled inventory for development, verify signed inventory in production, and reject any external package or manifest that attempts to self-assert built-in status.
- [ ] Keep all extension execution in host-supervised child processes. Reject in-process extension loading, Rust dynamic libraries, frontend plugins, WebView scripts, content scripts, Svelte components, stylesheets, remote UI entrypoints, and direct Tauri access.
- [ ] Treat extension output as bounded data only. Validate protocol frames, candidate payloads, declarative view nodes, settings contributions, icon references, action identities, revisions, sizes, and rates in Rust before publishing typed DTOs to the frontend.
- [ ] Route every extension view through shared Svelte components and semantic design tokens. Route every user action back through the typed Tauri bridge, Rust authorization, and versioned extension protocol to the owning extension.
- [ ] Add architecture checks that reject domain capability implementations outside `apps/extensions`, extension-specific components in the frontend, extension-supplied executable Web assets, and built-in-only runtime or protocol APIs.
- [ ] Add contract tests proving zero-extension startup, built-in and external equivalence, unsupported protocol rejection, malformed declarative content rejection, permission denial, action ownership, stale-revision rejection, process crash isolation, restart policy, disablement, and independent host operation after extension failure.

## Delete

Delete these source files and modules after the Tauri shell provides their required behavior:

- `main.rs`, `HostApp.rs`, `HostRunner.rs`, and `HostRunnerEvent.rs` as the desktop entrypoint, UI, and renderer owners. Move reusable orchestration out before deletion. The Tauri crate provides the only desktop entrypoint.
- `DesignPalette.rs`, `DesignSystem.rs`, `SearchInput.rs`, `SearchInputStyle.rs`, `SectionHeader.rs`, `InteractiveRowContent.rs`, `InteractiveRowStyle.rs`, `interactive_row.rs`, `detail_panel.rs`, and `settings_view.rs`.
- `render_preparation.rs` and renderer-specific selection, scrolling, texture, repaint, frame, viewport, animation, and layout state.
- `IconLoader.rs`, `IconLoaderCommand.rs`, `IconLoadResult.rs`, and GPU texture loading. Retain `IconIdentity.rs` and the extension-owned icon extraction and PNG cache.
- The renderer-specific `texture_name` method on `IconIdentity`.
- `OverlayMotion.rs`; motion state belongs to the frontend.
- `InvocationPresentation.rs`, `SettingsAction.rs`, `SettingsState.rs`, and their UI-state tests. Preserve bounded invocation output and typed settings behavior behind UI-independent Rust contracts, then implement presentation state in the frontend.
- Renderer preparation, render submission, frame submission, and native render visibility milestones in `ActivationTrace.rs` and their tests. Keep only UI-independent activation timing and add Tauri milestones.
- `SystemFont.rs`, `SystemFontFace.rs`, `system_font_paths.rs`, `system_font_paths_macos.rs`, `system_font_paths_windows.rs`, and renderer font-loading paths. The Tauri frontend uses the CSS system font stack.
- UI-specific tests, render-preparation tests, font-layout tests, widget tests, and renderer benchmarks.
- Direct dependencies on `accesskit_winit`, `egui`, `egui-wgpu`, `egui-winit`, `wgpu`, `winit`, `pollster`, and UI-only `fontdb` usage.
- The implicit `nanika-host` binary target and every deleted UI module declaration or export in the current `crates/nanika-host/src/lib.rs`. Keep `nanika-host` as a UI-independent library under the target `engine/runtime` boundary, remove obsolete manifest entries, and regenerate `Cargo.lock` from the new dependency graph.
- Shaders, renderer assets, cached build configuration, documentation, and packaging steps that exist only for those modules.
- `scripts/benchmark-native-windows.ps1`; replace it with Tauri desktop black-box harnesses for Windows and macOS.
- The `nanika-host` desktop-binary build and copy assumptions in `scripts/package-windows.ps1` and `scripts/package-macos.sh`. Rewrite only their desktop packaging paths for the Tauri application while retaining CLI, extension, signing, notarization, archive, and inventory behavior.
- `MacMenuTarget.rs`, `MacMenuTargetIvars.rs`, `MacNativeMenu.rs`, and `NativeMenu.rs` after the Tauri tray and menu implementation is validated. Remove Windows notification-icon and tray-menu ownership from `windows_instance.rs` while retaining its single-instance activation behavior.
- `HotkeyRegistration.rs` and its global event-dispatch glue after registration moves to `tauri-plugin-global-shortcut`. Retain the passive native timing observer and consume its matching delivery sample from the plugin's Rust callback without installing a competing `global-hotkey` event handler.
- `BuiltinCommandSpec.rs`, `BuiltinExtensionSpec.rs`, `builtins.rs`, and every compiled capability-specific registration path after equivalent extension manifests and host-owned distribution inventory loading are validated. Production must verify that inventory as part of the signed release. Do not replace these files with another hard-coded first-party registry.

Do not delete Rust behavior merely because it currently lives in a UI-owned file. First move search orchestration, extension coordination, platform events, settings commands, diagnostics, and lifecycle ownership behind UI-independent interfaces. Do not preserve any old UI type while doing so.

## Repository ownership

- [ ] Replace the current top-level technical layout with the business-owned layout defined in `tech-stack.md`. Complete this as one coherent repository reorganization before implementing the Tauri shell; do not maintain old and new paths in parallel.
- [ ] Move reusable UI-independent crates from `crates` into responsibility-named `engine` boundaries: `nanika-core` to `domain`, `nanika-host` to `runtime`, `nanika-platform` to `platform`, `nanika-protocol` to `extension-protocol`, `nanika-storage` to `storage`, `nanika-config` to `configuration`, `nanika-search` to `search`, and `nanika-extension-package` to `extension-management`. Preserve crate package names and public behavior unless a separate design task requires a change.
- [ ] Move `crates/nanika-cli` to `apps/cli`. Keep it a separate shipped process and Cargo workspace member.
- [ ] Move distributable extension processes into `apps/extensions/built-in/{application,command,script,calculator,clipboard}`. Move `nanika-extension-fixture` to `apps/extensions/fixtures/protocol` and `nanika-extension-acp-dummy` to `apps/extensions/fixtures/acp` so non-shipping executables cannot enter the release inventory accidentally.
- [ ] Create `apps/desktop/frontend` for Svelte presentation source, browser assets, frontend configuration, and mirrored frontend tests. Create `apps/desktop/shell` for the Tauri Rust crate, configuration, capabilities, isolation application, icons, native resources, and shell tests. Do not use a `src-tauri` directory or mix code across these boundaries.
- [ ] Keep the desktop `package.json` and `pnpm-lock.yaml` at `apps/desktop` as the single local frontend tool and Tauri CLI entrypoint. Point its scripts explicitly at `frontend`; keep one package and do not add a pnpm workspace.
- [ ] Move repository validation and benchmark scripts from `scripts` to `tooling/quality`. Move platform packaging scripts and inputs from `scripts` and `packaging` to `tooling/release`. Update all callers and documentation in the same change.
- [ ] Update root Cargo workspace members, build scripts, package scripts, Tauri `externalBin` paths, CI paths, benchmark paths, packaging inventories, ignore rules, and documentation for the new ownership boundaries. Reject references to the removed paths in an architecture check.
- [ ] Remove the obsolete top-level `crates`, `extensions`, `scripts`, `packaging`, and generated `dist` directories after their retained contents move. Keep generated frontend output untracked under `apps/desktop/frontend/dist`; keep Rust and release output under root `target`.
- [ ] Add an architecture check that fails when source appears in a top-level `rust`, `web`, `src-tauri`, `crates`, or `extensions` directory, when frontend source imports outside its typed bridge, or when an `engine` crate depends on Tauri or frontend packages.

## Tauri desktop foundation

- [ ] Add `apps/desktop` as the only desktop application. Start `apps/desktop/frontend` from the current official Vite `svelte-ts` template, remove its demo assets, and initialize the Tauri Rust shell manually in `apps/desktop/shell`. Do not introduce SvelteKit or a pnpm workspace.
- [ ] Add `apps/desktop/shell` to the root Cargo workspace and keep it as the only desktop binary target. Run Tauri commands from `apps/desktop`, where the CLI discovers the shell configuration without requiring the scaffold-default `src-tauri` name.
- [ ] Configure the desktop package scripts so `pnpm frontend:dev` and `pnpm frontend:build` operate only on `apps/desktop/frontend`. Set the shell's `frontendDist` to `../frontend/dist`, and use those scripts as Tauri's development and build hooks.
- [ ] Pin the newest mutually compatible stable Svelte 5, TypeScript, Vite, official Svelte Vite plugin, pnpm, Tauri 2, `@tauri-apps/api`, and test-tool releases. Commit the application-local `pnpm-lock.yaml`, record the pnpm version in its package metadata, pin the current Active LTS Node.js line for development and CI, and use exact direct dependency versions.
- [ ] Keep production frontend imports limited to compiler-emitted Svelte runtime modules and the required ESM surface from `@tauri-apps/api`. Add no Tauri plugin guest package unless the frontend owns a reviewed capability.
- [ ] Configure `svelte-check`, ESLint with Svelte support, and Prettier with Svelte support as required static-validation commands. Treat warnings and formatting drift as CI failures.
- [ ] Use the newest mutually compatible stable Tauri 2 core, CLI, build crate, official plugins, and JavaScript API. Review their release notes, security advisories, platform support, and generated configuration schema before pinning versions. Do not enable unstable or pre-release features.
- [ ] Move reusable host orchestration behind a UI-independent Rust API without changing search, storage, extension, or platform semantics.
- [ ] Complete the Delete list above and verify no removed dependency, source, test, asset, build step, or documentation reference remains.
- [ ] Define bounded `serde` request, response, snapshot, channel-message, and lifecycle-event DTOs for the Tauri boundary. Generate or contract-test matching TypeScript types.
- [ ] Register one typed Tauri managed-state handle for the UI-independent Rust services. Keep authoritative domain state in its existing owners and never hold a Tauri state lock across blocking work or an await point.
- [ ] Expose narrow Tauri commands for sessions, actions, and mutations through explicit application permissions. Validate command requests and scopes in Rust. Create each bounded Tauri channel through an authorized session command and bind it to the invoking window label. Deliver ordered, coalesced search snapshots, extension view updates, diagnostics, runtime state, and invocation output through those channels. Reserve Tauri events for small, low-frequency lifecycle notifications.
- [ ] Configure an explicit restrictive production Content Security Policy and window- and webview-specific Tauri capabilities. Bundle all frontend assets locally; keep the global Tauri object disabled; and deny remote code, arbitrary filesystem access, shell and process APIs, frontend event emission, development-only CSP allowances, and unvalidated paths.
- [ ] Enable the Tauri Isolation Pattern with a dependency-free classic-script isolation application that allowlists the exact frontend command surface and rejects invalid coarse payload envelopes. Measure its IPC overhead and keep Rust permissions, scopes, and validation authoritative.
- [ ] Enable `build.removeUnusedCommands` with standard static capability files. Avoid dynamically added ACLs and broad default permission sets, then verify release binaries contain no command excluded by the capability design.
- [ ] Add a validated `nanika-icon` custom protocol that accepts only required read requests, binds access to authorized window labels, and resolves opaque icon identities inside the Rust-owned cache without base64 payloads or frontend filesystem access. Serve only atomically completed variants as immutable content. Publish the fallback identity while extraction is incomplete or failed, never expose retry entries, and publish a new snapshot after successful completion. Keep Tauri's built-in asset protocol disabled.
- [ ] Declare built-in extension executables through `bundle.externalBin`, stage build inputs with the required target-triple suffixes, and keep all launch, containment, cancellation, and reaping inside the Rust process supervisor. Do not expose the Tauri shell plugin or process permissions to the frontend.
- [ ] Replace custom tray and menu implementations with Tauri `TrayIconBuilder` and `menu` Rust APIs. Replace direct global-hotkey registration and event dispatch with the official Tauri global-shortcut plugin Rust API. Install the retained passive timing observer from the shell and consume timing in the plugin callback. Grant neither feature to the frontend.
- [ ] Limit native tray and menu actions to Open Nanika, Settings, and Quit. Expose application refresh through an Application Extension command or view action, not through a shell-owned tray item or event.
- [ ] Implement hidden startup, logical-pixel active-monitor placement, summon, focus, dismissal, second-instance activation, tray or menu-bar actions, settings windows, and shutdown through the Tauri shell and platform adapters. Convert physical monitor geometry with the active scale factor.
- [ ] Set the macOS deployment target to 13.0. Record that launcher transparency requires `app.macOSPrivateApi` and therefore excludes Mac App Store distribution. Validate transparent-window startup and border behavior before enabling Windows-specific composition options or native shadow.
- [ ] Evaluate current stable Tauri native window effects as platform-specific progressive enhancement. Keep a complete semantic CSS fallback and accept an effect only after contrast, startup, focus, compositor, and fallback validation on physical Windows and macOS systems.

## Frontend architecture

- [ ] Implement and validate the product rules in `ui.md`; do not redefine interaction behavior inside individual components.
- [ ] Establish semantic design tokens for color, typography, spacing, size, radius, elevation, motion, and interaction states using plain CSS and CSS custom properties. Do not add Tailwind, CSS-in-JS, or a utility CSS runtime to the initial baseline.
- [ ] Implement accessible Svelte primitives for SearchInput, ResultList, ResultRow, SectionHeader, ActionBar, KeyHint, DetailPanel, EmptyState, LoadingState, and DiagnosticState without a component library.
- [ ] Keep Tauri calls inside one typed frontend bridge. Components consume typed application services and never import Tauri APIs directly.
- [ ] Use Svelte-local state and built-in reactivity. Do not add a router or global state framework; the bounded in-memory route stack and Rust snapshots remain the navigation and domain-state sources of truth.
- [ ] Use Svelte 5 runes and current component syntax only. Use `$derived` for derived state, keep `$effect` limited to external synchronization, and introduce no legacy reactive or event syntax.
- [ ] Wrap the application root and extension route content in `<svelte:boundary>` with safe recovery UI. Handle event-handler, promise, channel, and command failures explicitly because Svelte boundaries do not catch them.
- [ ] Prohibit `{@html}` for application and extension data. Map host-validated declarative rich content to shared Svelte components and render all other values through escaped text interpolation.
- [ ] Include the normalized operating-system locale in the initial Rust snapshot. Select bundled typed frontend message catalogs with deterministic English fallback and use browser `Intl` for locale-sensitive formatting without an i18n framework.
- [ ] Use CSS transitions for simple interaction states and Svelte's built-in transition, animation, and motion facilities for lifecycle-dependent motion. Do not add an animation library without a measured requirement.
- [ ] Use a semantic combobox and listbox model with `aria-activedescendant`. Keep DOM focus in the search input while Up and Down change the active option.
- [ ] Use browser scrolling as the source of truth. Reveal the active option only when it crosses the scrollport boundary, with no custom per-row scroll offset model.
- [ ] Virtualize only after measurement proves it is needed. The initial 100-result bound must remain smooth without continuous work while idle.
- [ ] Implement Root Search with localized title, optional subtitle, optional category, normalized icon, accessory text, pointer activation, Enter activation, input history, query selection on reopen, and stable publication while typing.
- [ ] Render extension List, Split, Detail, filters, pagination, nested navigation, Back, and typed actions from the existing declarative protocol. Use the same shared components for built-in and external extensions. Extensions never provide frontend code, styling, components, DOM behavior, or raw design tokens.
- [ ] Implement Settings from the bounded declarative settings contract while preserving JSONC as an advanced editing path.
- [ ] Define coherent empty, loading, degraded, error, and unavailable states before feature completion.

## Interaction and visual acceptance

- [ ] Approve Root Search and extension views from actual Windows and macOS captures at standard and high-DPI scale factors.
- [ ] Validate keyboard, pointer, focus, hover, pressed, selected, disabled, and destructive states with automated frontend tests and physical platform acceptance.
- [ ] Validate IME preedit, commit, candidate-window placement, query selection on reopen, caret geometry, Latin and CJK typography, and operating-system text scaling.
- [ ] Validate list boundaries and scrolling for keyboard, mouse wheel, trackpad, scrollbar dragging, and pointer selection.
- [ ] Validate accessible names, roles, active option state, live diagnostics, keyboard order, contrast, and reduced motion.
- [ ] Validate motion timing, easing, interruption, and reduced-motion behavior at 60 Hz and 120 Hz where available.
- [ ] Select a desktop E2E harness that drives a release-equivalent Tauri application on Windows and macOS. Tauri mocks and Browser Mode tests do not replace WebView2 and WKWebView acceptance. Any embedded WebDriver server or test-access plugin must be test-build-only and absent from release artifacts.

## Frontend testing

- [ ] Configure Vitest as separate projects: a Node project for pure TypeScript logic and Browser Mode projects using `@vitest/browser-playwright` for real Chromium and WebKit execution.
- [ ] Use the official `vitest-browser-svelte` renderer and Vitest browser locators, assertions, and interaction APIs. Do not add a simulated DOM environment or Testing Library by default.
- [ ] Keep Vitest, its browser packages, Playwright, and every test harness development-only and exclude them from production assets and runtime imports.
- [ ] Mock Tauri only at the typed bridge using the official Tauri mock APIs. Contract-test every request, response, channel message, lifecycle event, stale-generation rule, and error mapping against Rust fixtures.
- [ ] Test the complete extension presentation path from validated declarative fixtures through typed Tauri DTOs to shared Svelte components, then test typed actions returning to the owning extension identity. Do not mount extension-owned frontend code because none is allowed.
- [ ] Test every shared component through semantic roles, accessible names, keyboard and pointer behavior, focus, visible state, light and dark themes, reduced motion, loading, empty, degraded, and failure states.
- [ ] Use Vitest Browser Mode screenshots for bounded visual-regression coverage of stable shared primitives and launcher states. Keep operating-system captures as the authority for final visual acceptance.
- [ ] Cover Root Search typing, IME composition boundaries, Enter activation, Up and Down clamping, boundary-only scrolling, pointer activation, snapshot stability, query selection on reopen, icon fallback completion, and action state as component integration tests.
- [ ] Cover supported operating-system locale selection, English fallback, localized shell strings, locale-sensitive formatting, localized application titles, and original-name search aliases.
- [ ] Keep release-equivalent Tauri black-box tests and physical Windows and macOS acceptance as separate required layers. Browser Mode WebKit and Chromium coverage does not prove WKWebView, WebView2, native window, hotkey, tray, or packaging behavior.

## Cross-platform acceptance

- [ ] Validate focus, hotkey conflicts, active-monitor placement, mixed DPI, full-screen behavior, and elevated foreground windows on physical Windows and macOS machines.
- [ ] Use the system Evergreen WebView2 runtime without bundling a fixed runtime. Validate runtime absence, availability, version reporting, and clear native startup failure on clean Windows profiles. Validate WKWebView behavior on macOS.
- [ ] Validate startup enable, disable, repair, approval, stale paths, external disablement, rollback, and hidden idle launch on both platforms.
- [ ] Validate foreground and background second-launch behavior, stale instance recovery, shutdown cleanup, and per-user isolation on both platforms.
- [ ] Validate application, command, script, batch, clipboard, and macOS bundle actions on their supported platforms.
- [ ] Validate diagnostics for missing, incompatible, and failed built-in and external extensions on both platforms.

## Performance and release

- [ ] Add Tauri desktop summon, first-paint, focus, input-to-results, scrolling, memory, and hidden-idle measurements.
- [ ] Record emitted JavaScript and CSS bytes, chunk count, source-map absence, parse time, and evaluation time from every production frontend build. Investigate dependency or bundle growth before accepting it.
- [ ] Record P50, P95, P99, frame-time variance, long tasks, dropped frames, CPU, memory, database size, process count, and thread count on fixed Windows and macOS reference machines.
- [ ] Measure Isolation Pattern command and channel overhead under summon, typing, navigation, streaming, and hidden-idle workloads. Treat Brownfield as an explicit rejected fallback unless a reviewed decision changes the baseline.
- [ ] Run frontend unit and component tests, Rust workspace checks, cross-boundary contract tests, platform black-box tests, packaging checks, and binary inventory checks from clean trees.
- [ ] Extend `tooling/quality/check.sh` and `tooling/quality/check.ps1` with the selected frontend and Tauri checks while preserving their existing Rust workspace checks. Retain `tooling/quality/benchmark.ps1` for deterministic Rust benchmarks.
- [ ] Package only the Tauri desktop application, CLI, and built-in extension executables.
- [ ] Sign the Windows artifact and sign, notarize, and staple macOS artifacts with release credentials.
- [ ] Complete clean-profile extraction, first run, summon, settings, actions, diagnostics export, update, rollback, and removal acceptance for every release archive.

## Deferred work

- [ ] Remove `nanika-extension-acp-dummy` before 1.0 packaging.
- [ ] Add settings migrations and reset behavior only when a released format requires compatibility.
- [ ] Define machine overrides and secret handling before a capability requires them.
- [ ] Add maintenance snapshots and corruption recovery before the first post-release destructive database migration.
- [ ] Add captured output and launched-action process-tree cancellation only when a future capability requires them.
- [ ] Define and implement a platform-neutral paste-to-foreground host service with Windows and macOS adapters.
