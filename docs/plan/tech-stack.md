# Nanika Technical Stack

Status: current pre-1.0 baseline. Tauri is the only desktop UI solution. The Rust core remains authoritative.

## Selected baseline

| Area                       | Selection                                                                                                             | Boundary                                                                                                                                                                                                                                                                                                                                             |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Languages                  | Rust stable and TypeScript                                                                                            | Rust owns the core and privileged desktop boundary. TypeScript owns frontend presentation and local interaction.                                                                                                                                                                                                                                     |
| Platforms                  | Windows 10 and macOS 13 or later                                                                                      | These are the only supported targets and the only planned release targets. Linux and all other platforms are explicitly unsupported; keep platform code behind adapters and never use them as implicit fallbacks.                                                                                                                                    |
| Desktop shell              | Latest mutually compatible stable Tauri 2 ecosystem                                                                   | The Rust shell owns windows, IPC, capability configuration, custom protocols, tray integration, and application lifecycle. Review current Tauri releases and official guidance at every dependency update.                                                                                                                                           |
| Frontend                   | Latest mutually compatible stable Svelte 5 and TypeScript                                                             | One shared local frontend for Windows and macOS. Use plain Svelte as a client-only application with no SvelteKit, server rendering, remote code, or runtime CDN assets.                                                                                                                                                                              |
| Frontend build             | Vite and the official Svelte Vite plugin                                                                              | Build one local static application into `dist`. Tauri starts the Vite development server and consumes only the production build output in release artifacts.                                                                                                                                                                                         |
| Frontend package manager   | pnpm                                                                                                                  | Pin the pnpm version in the desktop application's package metadata, commit its `pnpm-lock.yaml`, and use exact direct dependency versions.                                                                                                                                                                                                           |
| Repository task runner     | just                                                                                                                  | Provide one cross-platform command surface for development and validation without taking ownership of toolchain installation or build logic.                                                                                                                                                                                                         |
| Frontend tool runtime      | Current Active LTS Node.js                                                                                            | Pin one development and CI version. Node.js is not an application runtime and must not appear in release artifacts.                                                                                                                                                                                                                                  |
| Frontend static validation | `svelte-check`, ESLint through `eslint-config-zoro` with Svelte support, and dprint                                   | Validate Svelte templates, component contracts, TypeScript, accessibility diagnostics, code defects, and deterministic formatting before production builds. Treat warnings as failures in CI.                                                                                                                                                        |
| UI platform                | Semantic HTML and CSS                                                                                                 | Use native text editing, DOM focus, browser scrolling, ARIA patterns, CSS custom properties, and system font stacks.                                                                                                                                                                                                                                 |
| Frontend localization      | Operating-system locale metadata today; typed local message catalogs and browser `Intl` planned                       | Rust includes the normalized operating-system locale in the initial application snapshot. The current frontend remains hard-coded English until bundled catalogs are implemented.                                                                                                                                                                    |
| WebView                    | System Evergreen WebView2 on Windows and WKWebView on macOS                                                           | Do not bundle a fixed WebView2 runtime in the portable MVP. Treat engine and operating-system versions as part of the validation matrix, fail clearly when the required runtime is unavailable, and do not depend on unsupported experimental web features.                                                                                          |
| Rust to frontend boundary  | Bounded Tauri commands and channels                                                                                   | Frontend uses `invoke` for requests; one long-lived Channel per WebView session delivers search state. Query RPC responses acknowledge submission without returning lists. Tauri events are reserved for small, low-frequency lifecycle notifications. Serializable DTOs and their TypeScript counterparts define the boundary.                      |
| Frontend Tauri API         | `@tauri-apps/api`                                                                                                     | Import only required ESM APIs inside the typed bridge. Keep the global Tauri object disabled and do not add frontend plugin bindings when Rust owns the capability.                                                                                                                                                                                  |
| Frontend security          | Tauri Isolation Pattern, capabilities, strict CSP, validated application commands, and validated custom protocols     | A dependency-free isolation application filters frontend IPC before Rust. Capabilities grant only required application, core, and plugin commands through explicit permissions. Command implementations validate requests and scopes. Bundle local assets only and deny arbitrary filesystem, process, shell, navigation, and remote network access. |
| Global hotkey              | Official `tauri-plugin-global-shortcut` Rust API                                                                      | The current fixed shortcuts are `Ctrl+Space` on macOS and `Ctrl+Alt+Space` on Windows. Registration stays in Rust and the frontend receives no global-shortcut permission. Runtime configurability remains planned.                                                                                                                                  |
| Fuzzy matching             | `nucleo-matcher`                                                                                                      | One persistent matcher owned by the named search owner thread.                                                                                                                                                                                                                                                                                       |
| Application paths          | `directories`                                                                                                         | Resolve roots once through `ProjectDirs`.                                                                                                                                                                                                                                                                                                            |
| Directory traversal        | `walkdir`                                                                                                             | Recursive scans without following symlinks by default or introducing a general parallel walker.                                                                                                                                                                                                                                                      |
| Windows discovery          | `windows` and `windows-sys`                                                                                           | Typed Shell COM plus direct known-folder, executable, and icon APIs.                                                                                                                                                                                                                                                                                 |
| macOS discovery            | `std::fs`, `plist`, and `objc2` AppKit/Core Graphics bindings                                                         | Localized application bundles, `Info.plist`, and normalized native icons.                                                                                                                                                                                                                                                                            |
| Icon cache encoding        | `png`                                                                                                                 | Deterministic RGBA PNG cache variants and fallback icons.                                                                                                                                                                                                                                                                                            |
| Clipboard                  | `clipboard-rs`                                                                                                        | Native Windows monitoring and measured macOS pasteboard polling.                                                                                                                                                                                                                                                                                     |
| Calculator                 | `fend-core`                                                                                                           | Deterministic, arbitrary-precision local evaluation in the calculator extension process.                                                                                                                                                                                                                                                             |
| Startup                    | `windows-registry` and `objc2-service-management`                                                                     | Current-user Windows Run entry and macOS `SMAppService.mainAppService`.                                                                                                                                                                                                                                                                              |
| Tray and menu bar          | Tauri `tray` and `menu` Rust APIs                                                                                     | Build and handle the tray entirely in the Rust shell. The current menu exposes Open Nanika and Quit; Settings is added only with the Settings surface. Domain actions such as application refresh belong to their extensions. The frontend receives no tray or menu mutation permission.                                                             |
| Single instance            | Nanika per-user Windows and macOS activation adapter                                                                  | A foreground second launch emits an activation event to the Tauri shell; a background launch exits without activation. The official plugin remains unsuitable until it guarantees the same per-user transport and activation contract.                                                                                                               |
| Serialization              | `serde`, `serde_json`, `jsonc-parser`                                                                                 | JSONC only for human-edited files and manifests. Internal APIs use typed Rust values.                                                                                                                                                                                                                                                                |
| Extension IDs and versions | `uuid` and `semver`                                                                                                   | UUID v4 for opaque IDs and Semantic Versioning for packages.                                                                                                                                                                                                                                                                                         |
| Database                   | SQLite through `rusqlite`                                                                                             | Default features disabled; only `bundled`.                                                                                                                                                                                                                                                                                                           |
| Background work            | Standard-library owner threads in the core                                                                            | Tauri runtime facilities stay at the shell boundary. No blocking core work runs on the Tauri event-loop or WebView main thread.                                                                                                                                                                                                                      |
| Process launch             | `std::process::Command` behind platform adapters                                                                      | Structured arguments by default; explicit shell mode only.                                                                                                                                                                                                                                                                                           |
| Errors                     | `thiserror` and standard error traits                                                                                 | Typed errors at crate and host boundaries.                                                                                                                                                                                                                                                                                                           |
| Diagnostics                | `tracing`, `tracing-subscriber`, and `tracing-appender`                                                               | Lossless local logging with backpressure. Record every operational failure and its concrete cause without deliberately attaching query or clipboard payloads.                                                                                                                                                                                        |
| Benchmarks                 | `criterion` as a dev dependency                                                                                       | Default features disabled; targets stay outside runtime crates.                                                                                                                                                                                                                                                                                      |
| UI acceptance              | Computer-use in the actual Tauri application                                                                          | Validate roles, accessible names, keyboard and pointer input, focus, CSS layout, and observable state in the platform WebView. Measure startup, search, delivery, and rendering through the real Rust-to-UI path. Automated tests cover Rust.                                                                                                        |
| Extension runtime          | Host-supervised native child processes                                                                                | Every domain capability runs through the same extension supervisor and versioned protocol. Built-in status provides no in-process, frontend, permission, lifecycle, or failure-handling shortcut.                                                                                                                                                    |
| ACP                        | Official `agent-client-protocol` SDK with `async-io`, `async-channel`, `async-process`, `futures`, and `futures-lite` | Stable ACP v1 only. One isolated supervisor thread drives each ACP process; `rustix` terminates the macOS process group. No project-wide executor.                                                                                                                                                                                                   |
| Extension package          | Native executable and declarative metadata in a ZIP with `.nanika` suffix                                             | `zip` default features disabled; only `deflate-flate2-zlib-rs`. Packages contain no frontend entrypoint and are never loaded into the WebView.                                                                                                                                                                                                       |
| Package integrity          | `sha2`                                                                                                                | SHA-256 for corruption detection.                                                                                                                                                                                                                                                                                                                    |

Use the latest mutually compatible stable releases when adding or updating dependencies. Commit `Cargo.lock`. Do not use Git dependencies, wildcard versions, or pre-release versions by default. Review each non-standard-library dependency for necessity, features, transitive cost, maintenance, and platform support.

Commit the desktop application's `pnpm-lock.yaml`, pin the pnpm version through its package metadata, and pin the current Active LTS Node.js line for development and CI. Use exact direct dependency versions. The initial frontend has no SvelteKit, router, global state framework, component library, utility CSS framework, CSS-in-JS runtime, animation framework, simulated DOM, icon font, analytics SDK, remote font, or remote asset dependency. Add one only after a demonstrated requirement and architecture review. Prefer platform APIs, Svelte-local state, semantic HTML, plain CSS, and static local assets.

The initial production frontend imports only Svelte runtime modules emitted by the compiler and the required ESM surface from `@tauri-apps/api`. Tauri plugin guest packages are absent unless a reviewed feature must be owned by the frontend. Vite, the Svelte Vite plugin, TypeScript, Node.js, pnpm, and static-analysis tools are development-only. Inspect the generated `dist` instead of inferring bundle contents from package metadata.

Use current stable HTML and CSS features supported by the minimum WKWebView baseline and the system Evergreen WebView2 runtime. Prefer CSS custom properties, logical properties, Grid, Flexbox, `color-scheme`, media queries, and feature queries over compatibility libraries or JavaScript layout. A newer platform feature must have an explicit fallback when support differs across the validation matrix.

Tauri serves a local static frontend. The desktop package scripts run from `apps/desktop`; `pnpm frontend:dev` starts the Vite project in `apps/desktop/frontend`, and `pnpm frontend:build` writes its production output to `apps/desktop/frontend/dist`. The shell config in `apps/desktop/shell` sets `frontendDist` to `../frontend/dist` and uses those package scripts for `beforeDevCommand` and `beforeBuildCommand`. The cross-platform repository entry points are `just dev` and `just check`; recipes only delegate to the existing package and quality commands. Every `just dev` first builds and stages the latest debug extension executables, then starts `tauri dev`; frontend-only edits use Vite hot reload, while Rust shell or extension edits require restarting that dev process. `pnpm --dir apps/desktop build:debug` exists only for a packaged debug `.app` or executable needed by desktop automation. It is not the ordinary launch path. Quality checks disable Rust incremental compilation, use a unique temporary Cargo target, and delete that target on success, failure, or handled interruption so tests, benchmarks, documentation, and cross-checks never accumulate in the development target. Before checking the shell, the same workflow builds the current extensions in that temporary target and replaces the fixed-size `target/tauri-binaries` sidecar set required by Tauri; old sidecars are removed before copying and cannot accumulate. Development keeps one reusable Cargo target with line-table debug information; ordinary `dev` never performs a full clean because that would turn every launch into a complete rebuild. `just dev` preserves state so persistence and restart behavior remain testable. `just dev-fresh` is the explicit destructive cold-start command: it refuses to run while Nanika is active, removes the product data root and Tauri development WebView data for the current platform, then starts the complete application. State deletion never runs inside ordinary `dev`. Development uses a fixed Vite server configured for Tauri, while production has no application server, server-side rendering, remote entrypoint, or runtime CDN dependency.

## Tauri adoption policy

Use the highest-level current stable Tauri 2 primitive that fully preserves Nanika's product, performance, security, and cross-platform contracts. Prefer Tauri core APIs first, official Tauri plugins second, and a narrow platform adapter only when the stable Tauri surface cannot express a required behavior. Use official plugins from Rust when the frontend does not need their authority, and do not install their JavaScript guest bindings or grant their commands merely for convenience.

The initial Tauri baseline enables these current stable capabilities:

- The Isolation Pattern with a dependency-free classic-script isolation application that allowlists command names and validates coarse payload envelopes before IPC reaches Rust. Rust command permissions, scopes, and validation remain authoritative.
- `build.removeUnusedCommands = true` with standard static capability files, no dynamically added ACLs, and explicit permissions instead of broad default permission sets.
- Tauri commands for request-response work, channels for ordered streaming, and events only for low-frequency Rust-to-frontend lifecycle notification.
- Tauri `WebviewWindow`, `tray`, `menu`, lifecycle, scale-factor, theme, and monitor APIs at the Rust shell boundary.
- Official `tauri-plugin-global-shortcut` through its Rust API, with no frontend permission.
- Tauri managed state for one typed shell handle to the UI-independent Rust services. Do not duplicate domain state in Tauri, use global mutable state, or hold a state lock across blocking work or an await point.

Isolation adds cryptographic IPC work. Measure its summon, query, navigation, and channel overhead in release builds, keep the isolation application free of third-party dependencies, and optimize the message shape or frequency if targets are missed. Replacing Isolation with Brownfield requires an explicit security and performance decision, not a silent fallback.

Current narrow exceptions are deliberate. Keep the custom per-user single-instance adapter because Nanika requires foreground-versus-background activation semantics and per-user transport behavior not guaranteed by the official plugin. Keep native startup integration because the current official autostart plugin uses LaunchAgent or AppleScript on macOS rather than `SMAppService`. Keep active-monitor placement, native hotkey timing observation, and process containment behind platform adapters because the higher-level Tauri APIs do not provide their complete contracts.

Do not enable unstable multiwebview support or another experimental Tauri feature in the product baseline. At each Tauri ecosystem update, review release notes, remove obsolete workarounds, adopt newly stable capabilities when they replace custom code without regression, and record any remaining exception in this section.

Stable Tauri native window effects may progressively enhance the launcher through platform-specific configuration. The semantic CSS surface remains complete without them. Select an effect only after physical Windows and macOS validation confirms text contrast, transparent-window startup, compositor cost, resizing, focus transitions, and fallback behavior.

The frontend uses Svelte 5 runes and current component syntax only. Use `$state` for local mutable presentation state, `$derived` for derived state, and `$effect` only for external synchronization that cannot be expressed by an event handler or lifecycle boundary. Do not use legacy reactive statements, legacy component event directives, or Svelte stores for component-local state. The current `<svelte:boundary>` wraps the application root, including extension routes. A separately scoped extension-route boundary remains TODO. Event-handler and asynchronous failures remain handled explicitly because Svelte boundaries do not catch them.

Never render application, extension, invocation, diagnostic, or settings content through `{@html}`. Render plain strings through normal Svelte interpolation and render rich content only from bounded host-validated declarative nodes mapped to shared components.

## Repository and workspace policy

Use a virtual Cargo workspace with `resolver = "3"` and Rust 2024 edition. The root `Cargo.toml` is Cargo-required project metadata, not Nanika user configuration. Share package metadata through `workspace.package`, share dependency versions through `workspace.dependencies` only when feature requirements match, and keep platform-specific features local. Inherit `workspace.lints` in every member. Keep one root `Cargo.lock` and one root `target` directory.

Organize source by product responsibility instead of creating top-level language buckets. `apps` contains executable product surfaces and executable test fixtures. `engine` contains reusable UI-independent application behavior. `tooling` contains repository-only quality and release support. `docs` contains the current design. Root files are limited to workspace metadata, repository policy, licensing, and version-control configuration.

Target layout:

```text
Cargo.toml
Cargo.lock
apps/
  desktop/
    package.json
    pnpm-lock.yaml
    frontend/
      index.html
      svelte.config.js
      tsconfig.json
      vite.config.ts
      src/
    shell/
      Cargo.toml
      build.rs
      tauri.conf.json
      capabilities/
      icons/
      isolation/
      src/
  cli/
  extensions/
    built-in/
      application/
      command/
      script/
      calculator/
      clipboard/
    fixtures/
      protocol/
      acp/
engine/
  foundation/
  runtime/
  platform/
  extension-protocol/
  storage/
  configuration/
  search/
  extension-management/
docs/
  plan/
tooling/
  quality/
```

`apps/desktop` is the only desktop application boundary. Its package manifest and lockfile coordinate frontend and Tauri commands, while implementation source remains separated. `apps/desktop/frontend` contains only the browser-realm Svelte presentation layer and frontend assets. `apps/desktop/shell` contains only the privileged Tauri executable, configuration, capabilities, isolation application, bundle resources, and native lifecycle integration. `frontend` and `shell` describe product responsibilities instead of implementation languages. `shell` replaces the scaffold-default `src-tauri` name.

`apps/desktop/shell`, `apps/cli`, and every executable below `apps/extensions` are members of the root Cargo workspace. `apps/extensions/built-in` contains extensions shipped with Nanika. `apps/extensions/fixtures` contains non-shipping executables used to verify the standard extension and ACP boundaries. `engine` members may depend on other `engine` members but never on Tauri, WebView, Svelte, browser, or desktop-shell types. The shell depends inward on `engine`; the frontend communicates only through bounded shell contracts. Extension executables depend on `engine/extension-protocol` and other approved engine libraries but never on the frontend or shell.

The frontend remains one local plain Svelte 5 pnpm project built by Vite. Do not introduce a repository pnpm workspace for a single package. The desktop-level `package.json` is command and dependency metadata, not a second frontend package. Frontend configuration points explicitly at `frontend`; generated assets stay in `apps/desktop/frontend/dist` and remain untracked.

Directory names and Cargo package names describe responsibilities: `nanika-foundation` lives in `engine/foundation`, `nanika-host` lives in `engine/runtime`, `nanika-protocol` lives in `engine/extension-protocol`, `nanika-config` lives in `engine/configuration`, and `nanika-extension-package` lives in `engine/extension-management`. `foundation` contains project identity, extension identifiers, and diagnostic primitives, with no domain capabilities. The `platform`, `storage`, and `search` names describe their corresponding responsibilities.

Do not create top-level `crates`, `extensions`, `src-tauri`, `web`, `rust`, `scripts`, `packaging`, or `dist` directories. Cargo's root `target` remains the single generated build tree; release archives and evidence belong under named subdirectories of `target`, not a new repository-root output directory. The runtime contains reusable orchestration and service logic or is divided into smaller UI-independent engine members when ownership becomes clearer.

## Cross-platform architecture

Shared core, frontend, diagnostics, protocol, configuration, storage, search, and extension lifecycle behavior is platform-neutral. The same frontend source and Rust contracts run on Windows and macOS. Platform-specific behavior exists only behind typed adapters in `nanika-platform`, the narrow Tauri shell boundary, or an extension's explicit platform adapter. A shared feature is not complete if it works on only one supported OS.

Every platform adapter contract must preserve the same user-visible semantics, failure boundary, cancellation behavior, and diagnostics shape on Windows and macOS. Platform implementations may use native APIs, but platform details must not leak into shared state or wire protocols. Linux and every other non-baseline platform are unsupported and must not enter shared paths or receive an implicit fallback. Adding another platform requires an explicit baseline update, adapter implementation, release decision, and validation matrix.

## Extension-first product model

Extensions are Nanika's only first-class domain capability unit. The bare host is deliberately useful as infrastructure but empty as a product capability provider. It can start, show Root Search, manage extensions, expose control-plane settings and diagnostics, and render an empty or degraded state, but it contributes no candidates, commands, content sources, or domain actions itself.

Application discovery, command execution, script discovery, calculation, clipboard history, agents, and every future domain capability must enter through the extension contract. Adding capability-specific logic to the frontend, Tauri shell, runtime, search engine, storage layer, or platform layer is an architecture violation. Shared infrastructure may provide a generic service only when it is capability-neutral and exposed through the same permission-checked host-service contract to every authorized extension.

The ownership boundaries are:

| Layer             | Owns                                                                                                                                     | Must not own                                                                                                                                     |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Shared frontend   | All launcher, Settings, diagnostic, and extension view UI, plus DOM, CSS, accessibility, local interaction, and presentation state       | Domain execution, extension-specific components, extension-supplied markup or code, privileged side effects, native tray or pre-WebView recovery |
| Tauri shell       | Window and application lifecycle, IPC, capabilities, custom protocols, tray, global shortcut integration, and native boundary wiring     | Domain capability logic, extension rendering, direct candidate contribution                                                                      |
| Engine            | Extension supervision, generic host services, search aggregation and ranking, storage, configuration, diagnostics, and platform adapters | First-party capability implementations, extension-specific presentation, Tauri or WebView types outside the shell boundary                       |
| Extension process | One or more declared domain capabilities, candidates, declarative views, configuration consumers, and typed action handling              | Host memory, another extension's state, DOM, WebView, Tauri APIs, native window handles, arbitrary frontend code                                 |

The only domain interaction path is:

```text
Extension process
  -> versioned extension protocol
  -> UI-independent Rust runtime
  -> session-bound Tauri Channel for search and view updates
  -> shared Svelte component
  -> WebView
```

User input and actions travel from Svelte through Tauri `invoke`, an authorized shell command, the Rust runtime, and the extension protocol to the owning extension. The Channel carries Rust-to-frontend updates only. The frontend may update ephemeral presentation state synchronously, but it never executes a domain action. No domain capability may bypass the extension process, protocol validation, Rust authorization, typed bridge, or shared renderer.

There is no first-party capability class. This follows the relevant VS Code extension-host model while retaining a single product-owned Web UI. Nanika does not adopt the Chrome Web Extension execution model: an extension package contains a native executable and declarative metadata, not a frontend bundle that is loaded into the WebView.

Extension classes are distribution classes only:

- `Built-in`: an extension executable shipped with the default Nanika distribution and enabled by default. It cannot be uninstalled because it belongs to that distribution.
- `External`: an extension executable installed from a `.nanika` package.

Both forms use the same complete `ExtensionManifest`, version negotiation, contribution types and validation, capability contract, lifecycle, permissions, host services, process supervisor, declarative view schema, action routing, failure policy, and diagnostics. Every extension under `apps/extensions/built-in` owns an ordinary `manifest.jsonc`; external `.nanika` packages carry the same file and schema. The shell embeds the reviewed built-in manifest set as its host-owned development inventory and derives each companion executable name from the active target entrypoint. Built-in identity comes only from that inventory, is not an extension-controlled field, and cannot be self-asserted by an external package. Built-in status grants no extra privilege, protocol surface, manifest field, in-process API, frontend component, or direct host registration.

A development launch builds every built-in extension, replaces the target-triple sidecar set, and then lets Tauri compile and run the desktop shell. The packaged Rust shell resolves companion executables from each validated manifest's active target entrypoint. Building or copying only the shell is an invalid development or packaging layout and produces feature-specific startup diagnostics.

### Process boundary

Every extension runs as a separate host-supervised child process. The host owns process creation, protocol I/O, explicit cancellation, shutdown, reaping, and documented transport resource bounds. It does not silently restart an extension or convert slow work into failure. On Windows, the host creates extension processes suspended, assigns their kill-on-close Job Object, and resumes them only after containment succeeds. On macOS, each extension starts in its own process group. Host APIs never expose host memory, SQLite connections, global configuration, or another extension's state. Built-in packaging never bypasses this boundary. The MVP does not provide an enforceable OS sandbox, so a child process retains the filesystem access of the current user.

Built-in extension executables are declared through Tauri `bundle.externalBin`. Build inputs use Tauri's required `-$TARGET_TRIPLE` filename suffix, while release inventory verifies the final platform bundle names and locations. The Rust process supervisor resolves and starts only validated bundled executables and retains the containment behavior above. The frontend receives no Tauri shell or process permission, and the Tauri shell plugin is not part of the frontend execution path.

Do not load extensions in-process or through Rust dynamic libraries. This is process and failure isolation, not a security sandbox. MVP extensions are trusted native code; enforceable isolation requires a future sandbox decision.

### Shared interaction

The product owns Root Search. Rust owns input history, search aggregation, contextual ranking, final ordering, execution, and durable state. The frontend owns the focused text field, active option, local keyboard interaction, and scroll presentation. An extension does not receive a Root Search row merely because it is installed. `contributes.commands` and `contributes.views` create static entries; presence-based `contributes.rootSearch` lets the extension publish bounded dynamic entries for the current query. An extension may combine these contribution points or use none of them. Calculator declares only `rootSearch`, so it appears only when its query detector returns a calculation. Clipboard declares one static View and no command. Extensions do not control cross-extension ordering.

Nanika follows VS Code's extension-point separation at the manifest boundary: commands are invocable operations, while views are independently registered UI contributions. Nanika does not adopt VS Code's UI provider implementations. A contributed View is opened by ID through the native extension protocol and returns bounded declarative data rendered by the shared Svelte frontend. It never supplies a tree widget implementation, Webview, HTML, CSS, JavaScript, or component code. Static and dynamic search candidates carry an explicit `action` or `view` entry type so Root Search can apply interaction policy without extension-specific checks.

A command may complete without a view or push a route-local declarative view. The extension supplies a bounded `ListView` or `DetailView`; the shared frontend owns pixels, typography, accessibility, keyboard behavior, focus, and platform consistency. A list may request the semantic `Plain` or `Split` layout, sections, selection, detail content, filters, pagination, and typed item actions. A standalone detail may declare actions; actions for a detail nested in a list belong to its selected list item. The extension never receives HTML, CSS, JavaScript, a DOM reference, a WebView handle, a native handle, or an arbitrary drawing surface.

An extension package cannot contain a frontend entrypoint, executable Web asset, stylesheet, Svelte component, WebView preload, content script, or remote UI URL. Bounded static presentation data such as text, metadata, and validated icon references crosses the extension protocol as data. The Rust runtime validates it, and the shared frontend maps it to product-owned components and tokens.

Each pushed view has an extension-scoped ID and monotonic revision. Rust validates every view document and serializes extension operations. Requestless invalidations retain at most one pending view identity per extension and run on a dedicated delivery worker, so an extension waiting on a view response cannot block the sole Root Search Channel writer. A completed invalidation mutates only the same route and revision; stale results are discarded. The frontend applies only matching revisions, keeps local text editing synchronous, coalesces outbound search changes while an authoritative update is pending, and routes selection changes directly. Back submits a typed request and retains the active route until Rust publishes the authoritative `Pop` snapshot; a failure stays visible on that route. The `Dismiss` navigation effect hides the launcher without closing extension routes or unmounting their frontend views. Summoning the launcher reveals the same route, with its query, selection, preview, scroll positions, focused control, and text selection intact. Only explicit back navigation (`Pop`) closes the active route; ending the WebView session releases the remaining routes in reverse stack order. Hidden views do not poll or run animation loops. Nested routes are bounded, and stale updates cannot mutate a different route.

`ViewActionStyle` communicates primary, secondary, or destructive prominence. It does not grant behavior or permission. Every action is rendered by the shared frontend and returned through a typed Tauri command to the owning extension. Host services such as clipboard writes and process launches remain separately permission checked in Rust.

## UI and interaction

The current shell creates one fixed 760 by 520 logical-pixel, undecorated, transparent, always-on-top Tauri `WebviewWindow` for the launcher. A separate decorated Settings window is planned but not configured. macOS transparency requires `app.macOSPrivateApi`; enabling it excludes Mac App Store distribution, so the planned macOS release path is direct Developer ID distribution with notarization. Set `bundle.macOS.minimumSystemVersion` to `13.0` instead of accepting Tauri's lower default. On Windows, the launcher native shadow is disabled. Evaluate `noRedirectionBitmap` only if measurements reproduce a transparent-window startup flash and confirm the option fixes it without regressions.

The primary process starts hidden with only its tray or menu-bar visible. Tauri loads the launcher frontend while hidden. A summon positions, shows, and focuses the existing window even when runtime initialization is still pending; the Channel then delivers ready or error state. Frontend interactive-readiness acknowledgement is not implemented. The shell owns visibility, placement, native focus, scale-factor changes, and application lifecycle. Tauri window size and position use logical pixels; platform adapters convert physical monitor geometry with the current scale factor before placement and re-evaluate it when the monitor or scale factor changes. The WebView owns text input, IME, DOM focus, layout, painting, and accessibility.

The shared visibility contract is `show`, `hide`, and `toggle`. The frontend never decides native visibility. Platform-specific focus, active-monitor placement, full-screen behavior, elevated-window behavior, and any required macOS ordering workaround stay behind the Tauri shell and `nanika-platform`. Hidden state must stop animations, repeating timers, observers that poll, and unnecessary event delivery. Outstanding startup and query operations remain pending until completion, explicit cancellation, transport closure, or disposal.

The frontend owns the visual language through plain CSS, semantic CSS custom properties, and reusable Svelte components. Global styles define reset, tokens, platform themes, and shared primitives; component styles remain scoped and consume semantic tokens without private product-specific color or spacing systems. Implemented reusable components cover Root Search rows, extension views, detail content, contribution icon tiles, and semantic content icons; the broader primitive list remains an incremental refactor, not an existing component inventory. Svelte's built-in transitions and motion primitives implement state-driven motion only where CSS alone cannot express the lifecycle.

The visual baseline is compact, neutral, content-first, and platform-aware without maintaining separate designs. Use the operating-system UI font stack through CSS. Do not bundle a font unless platform testing proves a missing glyph or metric defect that cannot be solved by the system stack. Text inputs use semantic HTML controls with native selection, caret, IME, and accessibility behavior. CSS controls size and spacing without replacing text editing. Global WebView focus and active decorations are reset. Editable controls add no focus effect beyond native caret and selection; the current explicit accessibility exception is a one-pixel inset `focus-visible` indicator for keyboard-only buttons and selects.

Root Search uses the ARIA combobox pattern with a listbox popup. DOM focus remains in the search input. Unmodified Up and Down change `aria-activedescendant`; Ctrl+Up and Ctrl+Down navigate input history. Enter invokes the active option. Unmodified Tab invokes an active View entry through the same primary action and does nothing special for an Action entry. Escape dismisses the launcher. Reopening selects a non-empty query. The active option is revealed with the browser scroll container only when its bounding rectangle crosses the scrollport boundary. Navigation clamps at the first and last option.

Animations use CSS transitions or the Web Animations API only when state-driven motion needs interruption. Every motion defines duration, easing, interruption, and reduced-motion behavior. Hidden UI has no active animation frame loop. Summon must never expose an empty document or stale view before interactive readiness.

The frontend renders at the WebView device pixel ratio and uses vector CSS or sufficiently large raster assets. Application icons are served through a validated custom protocol by opaque identity. The frontend cannot construct filesystem paths. The protocol accepts only the required read method, binds requests to an authorized window label, serves only completed cache variants as immutable content with an explicit MIME type, and rejects unknown identities, traversal, incomplete entries, and out-of-scope roots. Tauri's built-in asset protocol remains disabled because Nanika does not expose a general filesystem-backed asset surface.

The current desktop shell includes a minimal tray or menu-bar item:

- Windows notification-area tray icon.
- macOS `NSStatusItem`.
- `Open Nanika` and `Quit`.

Root Search accepts unmodified F5 only while its launcher is visible and focused. It requests protocol refresh from extensions declaring `rootSearch`, waits for their correlated completions off the UI thread, then republishes the current query through the session Channel. Application discovery remains inside the Application Extension. Extensions exposing only static command or View entries are not refreshed. F5 is neither a global shortcut nor a WebView reload, and it has no action inside extension views.

The Settings window is not implemented. The runtime already exposes host-owned snapshots for every enabled extension's static `contributes.configuration` declaration and accepts complete validated value snapshots; a future Settings window will render that existing contract. JSONC is the current advanced editing path.

The extension contribution is the sole authority for configuration property names, types, defaults, bounds, descriptions, and presentation hints. The host merges persisted values over contribution defaults, validates complete snapshots, and owns comment-preserving atomic JSONC persistence. Extension processes never read or write the configuration root. Nanika protocol initialization carries the complete effective snapshot. A saved change queues one request-correlated `configurationChanged` snapshot for a running Nanika extension, which later answers `configurationApplied` or a concrete error; queue admission is not application completion. Persistence success and runtime application success remain distinct. ACP receives the same snapshot in the namespaced `nanika.configuration` entry of standard `session/new` `_meta`; ACP has no live-update contract, so later changes apply to its next session. Host-level hotkey and reduced-motion settings are not implemented. Startup state remains OS-owned.

Rust resolves the operating-system locale at startup and includes its normalized language tag in the initial application snapshot. The current frontend does not consume it and renders hard-coded English. Bundled typed message catalogs, deterministic English fallback, and browser `Intl` formatting remain TODOs. Application display names are already extension-provided localized values with original-name aliases. No runtime translation download, localization framework, or frontend OS-information permission is included.

## Tauri application boundary

Rust is authoritative for domain state and side effects. The frontend is authoritative only for ephemeral presentation state such as input composition, active option, scroll position, open menus, and interrupted visual transitions.

The currently registered frontend-to-Rust commands are narrow and task-oriented:

- Open an authorized frontend session with a bounded state channel and return session metadata only.
- Publish the latest committed query through a coalesced slot; the RPC response acknowledges acceptance and never contains a result list.
- Acknowledge received search messages and close the session when the frontend is disposed.
- Invoke a candidate by stable extension, entry, and action identity.
- Submit a typed extension view event or Back request with route ID and revision.
- Dismiss the launcher.

Configuration snapshots and updates, diagnostics export, startup control, and application shutdown are not exposed as Tauri commands yet. Frontend readiness acknowledgement is also not implemented; current hotkey tracing ends at native delivery.

The current Rust-to-frontend Channel carries one `RootSearchSnapshot` state document, not imperative drawing instructions:

- Root Search state with session ID, frontend request ID, delivery revision, query, phase, result list, warnings, and a safe optional error.
- Navigation state with its own revision, current extension route snapshot, busy/error state, and dismiss counter.

The runtime already produces typed invocation-output and configuration-result batches, but the desktop shell does not consume or publish those batches yet.

Tauri Channels carry data from Rust to the frontend; they are not bidirectional WebSockets. Frontend requests use Tauri's existing `invoke` transport, not a new connection per keystroke. The frontend creates one Channel and its receive callback before calling `open_session`. Hiding or showing the launcher does not recreate it. A page reload creates a new session and replaces the old Channel. No application HTTP server, WebSocket server, or custom RPC transport is involved.

The implemented Root Search contract is:

| Operation            | Input                                                             | Response or effect                                                                                                                                                                                                     |
| -------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `open_session`       | Channel with receive callback already installed                   | Returns `sessionId`, `locale`, `maxQueryChars`, and the shell-owned `resourceOrigin`. Registers the Channel and starts an empty query. If runtime startup is pending, that query starts when the runtime is installed. |
| `publish_query`      | `{ request: { sessionId, requestId, query } }`                    | Returns success or error only. Accepts the newest request ID, ignores older submissions, and submits the query to the Rust search owner.                                                                               |
| Search Channel       | Rust-owned data                                                   | `{ sessionId, requestId, revision, query, phase, results, error }`, where `phase` is `searching`, `ready`, or `error`.                                                                                                 |
| `acknowledge_search` | `sessionId`, `revision`                                           | Confirms that the frontend received and handled the message; does not claim that the DOM has painted.                                                                                                                  |
| `close_session`      | `sessionId`                                                       | Releases only that session; a late close cannot detach a newer page.                                                                                                                                                   |
| `invoke_candidate`   | Current session/request IDs and extension, entry, action identity | Rust rejects stale selections and invokes only a current candidate.                                                                                                                                                    |

Frontend request IDs are increasing JavaScript-safe positive integers; request zero identifies the initial empty query. Rust maintains the core generation separately. Delivery revisions increase across the whole session. The frontend buffers Channel data that arrives before the session metadata response, accepts only its current session and latest request, and rejects old revisions. Input entered while the session is opening is submitted once it opens. Only Channel data updates search results; RPC acknowledgements cannot overwrite them.

A single named delivery worker reads the latest state and sends Channel messages. Core notifications only write to a one-slot wake queue and never perform WebView transport. There is at most one unacknowledged message per session; updates that arrive while it is in flight are coalesced in the current Rust state. On acknowledgement the worker rereads that state, so intermediate obsolete messages can be skipped without dropping the latest result. No thread or Channel is created per keystroke.

Search phases describe the available ranked results. A `ready` list can receive later replacements when an extension finishes initialization or background discovery; it does not claim that all icon extraction is complete. Runtime installation does not wait for extension initialization. Pending workers retain the latest query and publish into its generation when ready, without requiring another keystroke. Extension failures remain local warnings alongside healthy results; they never become a fatal launcher state. Application discovery publishes its current query again when searchable metadata is ready. Icons remain separate generated cache data and never gate searchable metadata.

Channel send failures and closure are explicit session transport failures. Rust logs queued delivery separately from acknowledged frontend receipt, using IDs, phase, counts, and elapsed time without query text or result titles. An unacknowledged message remains in flight and applies backpressure; it does not expire. The frontend does not impose a startup or query watchdog. Runtime initialization errors, transport closure, and render failures are visible instead of being converted into a permanent loading state. Reloading the page explicitly establishes a fresh session.

Commands never return borrowed core state, platform handles, filesystem paths, process descriptors, or unbounded extension payloads. The frontend never infers permission from presentation metadata.

High-frequency query and selection updates use latest-value coalescing. Actions, configuration updates, and other side effects are request-correlated and never replayed after an ambiguous failure. Closing the launcher cancels or detaches work according to the Rust lifecycle contract rather than leaving frontend promises as owners.

Tauri capabilities are window- and webview-specific and grant only the application, core, plugin, and event API permissions each surface uses. Application commands define explicit permissions and validate request scopes in their implementation. Event names, event payloads, and individual channel messages do not provide a per-message authorization boundary. An authorized session command therefore creates each bounded channel, binds it to the invoking `WebviewWindow` label, and sends only data allowed for that session. Production builds disable developer tools, remote navigation, arbitrary URL opening, drag-and-drop file access, the shell plugin, process APIs, the global Tauri object, and the built-in asset protocol unless an approved feature requires them.

The current desktop shell registers no application Tauri events. If a later small, low-frequency, non-sensitive Rust-to-frontend lifecycle notification needs multi-consumer delivery, a consuming surface receives listen permission only; frontend-to-Rust work still uses commands and the frontend receives no event emit permission. Search snapshots, extension views, configuration state, diagnostics, and invocation output must not use the event system. Do not evaluate generated JavaScript to transfer application state.

Configure the production Content Security Policy explicitly. It permits only bundled frontend resources, the minimum Tauri IPC transport required by the generated application, and `nanika-icon` image responses. Let Tauri inject the hashes and nonces required by bundled assets; do not allow remote origins, `unsafe-inline` or `unsafe-eval` for scripts, or broad filesystem-backed asset sources. Development-only allowances must not enter the production configuration.

The Isolation filter must allow Tauri's exact internal Channel fetch command, `plugin:__TAURI_CHANNEL__|fetch`, including its null payload, as well as the explicitly authorized application commands. Tauri 2.11.5 sends JSON messages below 8,192 bytes directly and fetches larger messages through that command. This threshold is an implementation detail to check on dependency updates, not a product payload limit. Rejecting the fetch blocks a large message and consequently every later message waiting behind its Channel sequence number. Do not work around it by truncating results, splitting lists below the threshold, disabling Isolation, or broadly permitting plugin commands.

Authoritative transport references: [Tauri Channel guide](https://v2.tauri.app/develop/calling-frontend/#channels), [Isolation Pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/), and the pinned Tauri `src/ipc/channel.rs` implementation. The single-session flow and receive acknowledgements above are Nanika's business contract, not requirements imposed by the Tauri examples.

## Search and ranking

The named search owner thread owns a persistent `nucleo-matcher` instance, aggregation, usage state, final ranking, and generation-tagged snapshots. The Tauri command boundary replaces a coalesced latest-query slot and wakes the bounded owner queue, so saturation cannot drop the current input. Each extension has a fixed protocol worker with a latest-query slot. A new generation waits for the initial snapshot from every ready extension before publishing, which prevents the result list from flashing through partial extension states. Later incremental snapshots replace one coalesced pending channel message, and stale request IDs and generations are discarded.

The `nanika-search` crate implements Unicode lowercase, punctuation-separated, whitespace-collapsed exact, prefix, token, fuzzy, empty-query, and alias matching through `nucleo-matcher`. Extensions may supply localized names as aliases. Candidate search values are normalized once when snapshots enter the host. The Tauri request boundary rejects queries above 4,096 Unicode scalar values as an explicit resource validation error. Fuzzy results require 12 score points per normalized query character. Contextual frequency and seven-day recency decay apply only inside the same lexical tier. Ranking sorts and returns every matching candidate received from extensions. Root Search must not silently truncate browsing results; rendering optimizations must preserve access to the complete catalog. Candidates are deduplicated by extension, entry, and action identity without a hidden per-extension result cap.

Clipboard history contributes one static `Clipboard History` View to Root Search and does not publish retained clipboard entries there. Opening the View returns its route-local `ListView` with a split detail pane, bounded local filtering, content-type filtering, selection, pagination, and typed actions. Clipboard content remains local and never enters diagnostics. `Clear` freezes the IDs matching the current type and search text before pagination, then deletes only those IDs in one SQLite transaction through the clipboard owner. Newly captured IDs outside that request remain. The owner reloads shared history after commit and removes only orphaned generated image payloads; cleanup failures are reported without reverting the committed history state. The action does not delete source files referenced by file entries.

Ranking order is deterministic:

1. Exact match.
2. Prefix match.
3. Token-prefix match.
4. Fuzzy match above the relevance cutoff.
5. Within a tier, bounded query-contextual frequency and recency boosts.
6. Fuzzy score, alphabetical title, and stable identity tie-breakers.

Global popularity cannot outrank a better lexical tier. Usage identity is `(extension_id, entry_id, action_id, query_context)`. History and usage keys lowercase and trim input while preserving punctuation significant to commands and scripts. Input history preserves the current draft. Empty-query launches update usage without creating an empty history entry. The storage owner is authoritative for usage: it commits first, then updates in-memory ranking. Usage and history are local-only and remain until an explicit user reset; there is no automatic age or count retention policy. No SQLite work occurs in the summon or per-keystroke path.

## Paths, configuration, and generated data

Use `ProjectDirs::from("com", "nanika", "nanika")` as the current identity. It produces the macOS bundle identifier `com.nanika.nanika`. This is a reasonable pre-1.0 default and may change if implementation evidence warrants it.

`data_local_dir()` is an API method, not a literal directory name:

| Product root      | Windows                             | macOS                                             |
| ----------------- | ----------------------------------- | ------------------------------------------------- |
| `<app-data-root>` | `%LOCALAPPDATA%\nanika\nanika\data` | `~/Library/Application Support/com.nanika.nanika` |

The default layout keeps all owned data under one product root while making user-owned data a distinct subtree:

```text
<app-data-root>/
  bootstrap.jsonc
  user/
    nanika.jsonc
    extensions.jsonc
    extensions/<extension-id>/settings.jsonc
    machines/<machine-id>/
  profile/
  databases/
    nanika.db
    extensions/<extension-id>.db
  extensions/<extension-id>/<version>/
  backups/config/
  backups/databases/
  logs/
  payloads/<extension-id>/
  cache/
    icons/<extension-id>/<icon-key>/{32,64,128,512}.png
    metadata/
```

`bootstrap.jsonc` contains only the effective config-root locator and machine ID. The default effective config root is `<app-data-root>/user`; an explicit absolute relocation remains supported. User configuration is the only syncable tree:

```text
<effective-config-root>/
  nanika.jsonc
  extensions.jsonc
  extensions/<extension-id>/settings.jsonc
  machines/<machine-id>/
```

Only the effective config-root tree is intended for Dropbox or another file-level sync service. Never synchronize `<app-data-root>` as a whole. Databases, indexes, clipboard content, extension artifacts, logs, backups, and caches are machine-local generated data and are never synchronized. Never synchronize live SQLite files.

Generated data is never deleted, pruned, rebuilt, or substituted automatically unless a feature-specific policy is documented and user-configurable. Clipboard history is the only current exception and applies its declared count and age policy transactionally. Corrupt or inaccessible configuration, SQLite databases, clipboard payloads, icon indexes, and extension artifacts fail explicitly with the concrete cause. A future cleanup or repair operation must be user initiated, scoped to named generated data, and specified before implementation. User configuration is never treated as disposable generated state.

Use JSONC for human-edited configuration and manifests. Parse through `serde` and `jsonc-parser`, keep CST types private, and convert to typed Rust values at the boundary. UI edits use targeted CST changes, preserve comments and formatting, reparse and validate, then replace files atomically. Each settings file has a `formatVersion`. Before the first release, format changes rewrite the only baseline and incompatible development state is reset. Do not carry pre-release migration code. Define a released-format migration policy only when the first released format change requires it.

The current `nanika-config` boundary implements bootstrap creation, absolute relocatable config roots, typed JSONC parsing, comment-preserving top-level and nested-object changes, atomic replacement, and path-preserving backups. Backups are data-safety artifacts and are never selected automatically as substitute configuration. Only an explicitly missing file selects defaults; corrupt content, metadata failures, and access failures remain errors.

Each extension configuration file has the exact shape `{ "formatVersion": 1, "values": { ... } }`. The host registers only extensions that declare `contributes.configuration`; a missing file uses contribution defaults, stored values override defaults, and validation always evaluates the complete effective object. Unknown keys, missing declared keys after merging, invalid types, and out-of-bounds values fail explicitly. Saving serializes validation, atomic persistence, and replacement of the in-memory snapshot under one registry lock so concurrent requests cannot make disk and memory describe different saved values.

The supported schema is a bounded JSON Schema-inspired subset, not general JSON Schema. One contribution has a non-empty title, 1 to 64 properties, and at most 1 MiB of encoded schema. Keys begin with lowercase ASCII and contain only ASCII alphanumerics, `.`, `-`, or `_`. Values support Boolean, Integer, String, Array, and Object. Integer schemas require inclusive `minimum`, `maximum`, and positive integer `multipleOf`; divisibility is anchored at zero, so validation is `value % multipleOf == 0`, independent of `minimum`. Strings are limited to 4,096 UTF-8 bytes and may declare the `path` presentation hint, which does not itself validate filesystem semantics. Arrays require `items` and `maxItems` from 1 through 5,000. Objects require 1 to 64 declared properties and may declare `required`; unknown properties are rejected. Schema nesting is bounded to four descendant levels. Extensions remain responsible for domain validation such as absolute paths after receiving the host-validated snapshot.

## Diagnostics

The host uses stable diagnostic codes and categories. `HostDiagnostic` keeps a concise user message separate from the complete technical source chain retained by Rust application state. The frontend receives the bounded user-facing diagnostic contract, while operational logs retain the code, category, operation, safe context, and concrete error chain needed to explain the failure. Nanika does not deliberately attach query text, clipboard content, configuration values, or extension payloads to diagnostics. Operating-system, library, protocol, and extension errors are not erased merely because they originate outside the host.

User-visible diagnostics name the unavailable capability and state what happened in plain language. The complete technical cause remains available through diagnostics. Independent failures remain independent; the runtime does not suppress them because their text or code happens to match.

The primary host is the only log owner. It writes INFO-and-higher lifecycle and failure events under `<app-data-root>/logs`; verbose mode adds DEBUG events. The non-blocking writer uses backpressure rather than discarding accepted records. Daily file rotation changes the active filename but does not delete old files. The first release has no duplicate suppression, byte budget, file-count retention, or startup log cleanup. Orderly shutdown flushes the writer.

`nanika-cli diagnostics <output.zip>` exports version and platform metadata plus every Nanika-owned daily log file currently present. It rejects symlinks and unrelated files, publishes through a same-directory atomic no-clobber hard link, and can run while the host is active. It does not silently omit older or larger owned logs.

## SQLite storage

Use one host database and one database per extension. Every database has exactly one writer that owns its connection and transactions. The host storage owner owns `nanika.db`; each extension owns its database through a named owner thread in its process. Connections never cross process boundaries. Extensions resolve their database path from the host-supplied data root and own their current schema definitions.

`nanika.db` baseline tables:

- `extensions(extension_id, kind, version, install_path, package_digest, state, updated_at)`
- `input_history(id, normalized_query, display_query, last_used_at)`
- `usage_stats(extension_id, entry_id, action_id, query_context, execution_count, last_executed_at)`

The host, application, and clipboard schemas use the current pre-release baseline directly. Host usage identity includes extension, entry, action, and query context; the composite primary key uses a `WITHOUT ROWID` table. Input history has an index matching its last-use ordering. Built-in extension rows cannot contain package metadata; external rows require one active version, install path, digest, and enabled or disabled state. Startup parses and validates extension metadata one row at a time, reports every malformed row, and continues with valid extensions as an explicit partial-capability state.

Application extension baseline tables:

- `scan_state(id, generation, status, started_at, completed_at, last_error)`
- `app_entries(entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key)`

Clipboard extension owns its content, hash, capture timestamp, and payload fields. Calculator is stateless in the MVP.

The clipboard schema stores typed text, file-list, or image references with content hashes, byte size, and capture timestamps. A row-level constraint permits exactly one payload column matching its content kind. Its current distribution contribution declares `clipboard.maxEntries` with default 50 and `clipboard.maxAgeDays` with default 7. The owner applies both limits transactionally at startup, after a successful capture, and after a live Nanika configuration update. Removed image rows and their extension-owned PNG payloads are reconciled as one owner operation. Image PNG payloads live under `<app-data-root>/payloads/com.nanika.clipboard`, outside synchronized configuration and SQLite.

Every database initializes the current embedded pre-release schema with idempotent DDL, strict tables, domain constraints, and `PRAGMA user_version=1`. Schema changes rewrite this only baseline and require incompatible development databases to be reset. No pre-release migration table, runner, compatibility reader, or schema-version history is retained. Enable `foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, and `busy_timeout=100 ms`. Design transactional migrations, rollback requirements, and consistent snapshots only when a released schema first requires compatibility.

## Threads and process execution

Use named owner threads for storage, application discovery, search aggregation, and platform event sources. Runtime configuration and storage initialization stay off the Tauri event-loop and WebView main thread. The search owner reuses one `nucleo-matcher` instance. Do not create a thread per query, action, or database operation. Fixed extension workers publish typed snapshots and carry generation IDs. Each active ACP extension owns one additional named supervisor thread for its isolated async protocol executor. Shutdown stops extension workers, storage, search, and platform events in that order. Every protocol receive wait, including initialization and configuration application, observes the shared application shutdown signal at a 25 ms polling interval on the worker thread. This interval checks explicit termination and never imposes a completion deadline.

Only the host process launcher and extension supervisor may create child processes. Extensions submit typed `Program`, `Shell`, `MacApplication`, or `WindowsApplication` descriptors. `Program` keeps structured arguments separate, with an explicit Windows raw-argument representation. `WindowsApplication` opens an absolute executable or original `.lnk` with the default Windows Shell verb, preserving shortcut metadata and native elevation prompts. Cancelling elevation returns the native error to the action; Nanika itself stays unelevated. `Shell` selects `cmd.exe` on Windows and `/bin/zsh` on macOS. Program and shell-command launches are detached with null stdio. Native application activation uses the operating system's launch semantics, and action success means the launch was accepted, not that a new process was necessarily created. One bounded launcher owner serializes spawn work; senders block to apply backpressure and accepted launch requests are never discarded. Windows releases detached process handles; macOS reaps children through `kqueue` `NOTE_EXIT` events without polling. Captured execution and launched-action process-tree cancellation remain a later descriptor mode.

## Platform adapters

### Single instance

Nanika runs one host instance per user session. Windows uses `Local\com.nanika.nanika` through `CreateMutexW`; a blocking platform event thread owns only the hidden activation window. macOS holds `nanika.instance.lock` with `flock`; a blocking platform event thread owns a local Unix datagram socket under `<app-data-root>`. One-byte activation and stop datagrams cannot leave the listener blocked on a partial stream connection. Both adapters feed bounded platform events to the Tauri shell and tolerate the primary's startup handoff race. A foreground second launch requests activation, then exits. A background second launch exits without activation.

### Global hotkey

The desktop shell owns registration through the official `tauri-plugin-global-shortcut` Rust API and forwards typed events into the Rust application service. The current fixed shortcut is `Ctrl+Space` on macOS and `Ctrl+Alt+Space` on Windows. A press toggles the launcher; second-instance activation always shows it. Runtime replacement of the shortcut is not implemented. Keep media keys outside the MVP. When configurability is added, registration conflicts and failed replacement must preserve the previous working shortcut and produce diagnostics.

Measure native hotkey delivery through a passive `nanika-platform` observer before the plugin's `global-hotkey` backend discards the source timestamp. The observer must always continue native event propagation and must never become an alternate hotkey delivery path. Use Carbon `EventTime` on macOS and `MSG.time` on Windows, then expose only a platform-neutral `Duration` to the host. Missing native timing must mark a sample incomplete instead of silently treating callback time as input time.

### Application discovery

The application extension scans standard platform roots and user-configured roots with `walkdir`. Do not follow symlinks by default. It scans at startup, on explicit refresh, and after a live configuration change. Persist generated metadata in the application extension database. Keep filesystem access out of the search hot path.

Windows uses known folders and native `IShellLinkW` resolution. macOS scans `.app` bundles, reads `Info.plist`, and resolves the display name against the operating system's preferred languages, including `InfoPlist.loctable`. The localized display name is the title while bundle names and file names remain searchable aliases. Paths are refreshable metadata; bundle IDs and resolved executable identities provide stable identities.

The application extension runs as its own process with one discovery owner. The host registers it through the universal worker path; explicit refresh is cancellable and stays off the Tauri event-loop and WebView main thread. A missing extension configuration file selects contribution defaults. An unreadable or invalid file prevents that extension process from launching and produces a concrete diagnostic without modifying its existing index. For a live change, the extension replaces its in-memory configuration, queues a correlated scan, and sends `configurationApplied` only after that scan publishes searchable candidates. Queue or scan failure restores the prior in-memory configuration and returns `configuration_apply_failed`; the already persisted host snapshot remains a separate outcome.

Windows discovery resolves every `.lnk` through Shell COM and validates PE targets before indexing. Validation is reused while canonical path, size, and modification time remain unchanged; benchmarks separate cold validation from warm refresh. Identity uses the canonical executable, effective working directory, and typed arguments. Shortcuts with custom elevation, window-state, installer, or compatibility activation settings also retain their source identity so they cannot merge with a direct executable or a shortcut with different behavior. A successful complete scan replaces the previous application rows inside one transaction. Cancelled, failed, or partial scans upsert discovered entries while preserving unseen data and report their exact state. No stale marker or delayed-deletion state is persisted. SQLite commits each generation atomically. Every indexed candidate remains available to host ranking; discovery does not switch to an undocumented top-k path at a catalog threshold.

Searchable metadata publishes before icon extraction. The Host sends the final ranked first ten entry IDs through the platform-neutral `PrepareEntries` protocol hint. The discovery worker prioritizes those missing icons, then populates the remainder in background batches. A complete persistent cache bypasses native icon acquisition and image processing. The hint has no response and never gates search publication or rendering.

- macOS 13+ calls `NSWorkspace.iconForFile` with the application bundle path. This API returns the system-resolved application image. Draw its `NSImage` once into a transparent 256 px sRGB Core Graphics bitmap, convert premultiplied channels to straight RGBA, and generate the missing 32, 64, and 128 px PNG variants from those pixels. System-supplied corners, plates, highlights, and shadows remain part of the image. Their appearance follows the running macOS version; macOS 26's automatic reshaping is not a guarantee for older releases.
- macOS cache identity includes rendering version, OS version, bundle and contents metadata, `Info.plist`, the main executable, `Assets.car`, the declared icon resource, and a custom `Icon` file. High-resolution modification/change times and file identities detect replacement and custom-icon metadata changes. Optional missing resources are fingerprinted as absent. Resource files are inspected for cache invalidation, not decoded as an alternate icon source.
- Windows retains native Shell item acquisition and resource selection at each requested cache size. Shell bitmap PARGB and icon-drawing BGRA are converted from premultiplied channels to straight RGBA before shared normalization; legacy icons without alpha retain dual-background alpha recovery. Shortcut cache identities cover the shortcut, executable, and configured resource stamps.
- Shared normalization crops only fully transparent outer margins, using every pixel with nonzero alpha to find bounds. It preserves opaque colors, partial alpha, details, and shadows, then fits artwork proportionally along the cache canvas's longest axis. Only aspect-ratio centering leaves space on the shorter axis. Nanika adds no occupancy inset, icon plate, or corner mask.
- The frontend requests the 128 px variant through `nanika-icon` and displays it at 26 CSS px. It performs no platform-specific image processing and relies on immutable HTTP-style cache identities. Placeholder images are transparent document outlines. System-provided generic icons are valid native images, not extraction failures.

A candidate publishes the shared placeholder identity until every required variant is complete. Incomplete markers and staged variants are never protocol-visible. Successful extraction completes the cache and publishes a new candidate snapshot; a changed identity produces a new URL without overwriting a previously served complete image. Concrete extraction failures are logged and remain placeholders until a later explicit scan succeeds. There is one native acquisition path per supported platform, and no raw bundle-image or TIFF conversion path on macOS.

All filesystem access, SQLite work, native acquisition, image processing, and cache writes stay off the Tauri event-loop and WebView main thread. Scans do not prune old caches, and SQLite corruption remains an explicit extension failure rather than triggering an automatic rebuild. Optional macOS roots do not make a scan partial; bundle executables require executable permissions. Application actions submit persisted typed launch metadata to the common host service. Full Windows and macOS release acceptance remains required.

### Clipboard

File entries obtain native artwork through `nanika-platform::FileIconCache`. Native calls and shared alpha-only normalization live in `engine/platform`, with independent macOS and Windows adapters reused by the Application Extension. Clipboard view rendering performs in-memory worker lookups only. After a response, it copies at most the first three paths required by the selected collection preview and the first path from each other visible file row under the history read lock, then schedules missing icons on a dedicated extension worker with the selected entry first. The worker progressively fills a bounded 512-path queue instead of rejecting an entire batch. Completed icons emit a requestless `ViewInvalidated` signal; the Host retains at most one pending invalidation per extension and requests a fresh declarative view on a worker separate from Root Search delivery. Cache variants are 128 px for 26 CSS px rows and 512 px for detail previews. Views load images through validated `IconReference` values. Full file paths are separate display-only text, never image sources or filesystem permissions; missing files or unavailable icons retain the semantic file artwork. The generated icon cache is separate from clipboard image payloads and is not deleted by `Clear`.

The clipboard extension captures permitted text, file lists, and images. Windows uses native change delivery. macOS checks `NSPasteboard.changeCount` through `clipboard-rs` at a measured 250 ms interval because the selected macOS API is polling-based; this background platform adapter is the documented exception to the no-polling default and never runs on the WebView thread. The watcher applies backpressure to a single owner responsible for capture, deduplication, SQLite persistence, configured retention, and orphaned extension-owned PNG cleanup. Resource validation rejects oversized content explicitly instead of skipping or truncating it: text and encoded file lists are limited to 1 MiB, file lists to 256 paths, and PNG images to 16 MiB, 8,192 pixels per dimension, and 16,777,216 pixels. Image previews use extension-scoped, content-addressed PNG names. The shared platform reader revalidates payload-root confinement, encoded size, PNG headers, dimensions, and pixel count before the shell serves a resource through the platform-specific custom-protocol origin; only validated immutable responses receive long-lived caching. Only native external clipboard changes trigger capture and persistence. Clipboard writes requested by an extension return the native clipboard revision, allowing the clipboard extension to filter its own write without reading, encoding, persisting, or reordering that content. Explicit refresh acknowledges current persisted state and does not recapture unchanged clipboard content. Worker errors are reported through the protocol. The implemented action uses the common host clipboard service, is labeled `Copy to Clipboard`, and closes the view after a successful copy. TODO: define a separate platform-neutral paste-to-foreground host service with Windows and macOS adapters before presenting paste behavior. Clipboard content never enters diagnostics or synchronized configuration.

### Calculator

The calculator extension uses `fend-core`. It declines plain search text and standalone values before evaluation, contributing only when the query contains an explicit symbolic or word operator. Evaluation runs in its extension process after the shared 4,096-character request validation and has no hidden evaluation deadline. The MVP context is deterministic and stateless. Successful results copy through the common host clipboard service.

### Command and script

The command extension contributes only for queries beginning with `>` and submits the remaining text as an explicit `Shell` descriptor. The script extension consumes the host-validated `script.entries` snapshot; every entry names an absolute interpreter, script path, structured arguments, and optional working directory. Its current distribution contribution defaults to an empty array. Neither extension reads the configuration root or creates child processes directly.

### Startup

Windows uses a quoted absolute executable path under the current-user `Run` key. macOS uses `SMAppService.mainAppService` with a minimum supported version of macOS 13. Startup launches Nanika hidden and idle. The operating system remains the source of effective registration state.

Startup status and mutations run through a bounded platform owner and report their effective state back to the host. Windows treats an unexpected existing Run value as needing repair. macOS preserves `RequiresApproval` and `NotFound` instead of collapsing them into a Boolean; approval opens Login Items rather than repeating registration.

The Tauri shell creates the tray and menu with `tauri::tray::TrayIconBuilder` and `tauri::menu` on both platforms. The current Rust-owned menu handles only `Open Nanika` and `Quit`; a left click toggles the launcher. Settings is not present until its window exists. Tray behavior never crosses into frontend authority and never exposes a domain capability action.

## Extension protocol and package

Nanika protocol v1 uses stdin and stdout with a 4-byte little-endian length prefix, an 8 MiB maximum frame, and a UTF-8 JSON object. ACP v1 uses its standard newline-delimited JSON-RPC 2.0 stdio transport with an 8 MiB frame limit in both directions. The two wire protocols never share a stream.

`ExtensionRuntime` is the common supervisor entry for built-in and external extensions. It selects the wire adapter from validated runtime metadata without changing permissions, lifecycle, or failure policy. Invocation outcomes are `Completed`, `Cancelled`, or `Failed`; only completion records usage, and cancellation is not shown as a failure. A process exit or protocol failure is reported explicitly. The first release does not restart, retry, or replay extension work automatically.

The Nanika adapter provides typed frames, a configuration-bearing initialization handshake outside the Tauri event-loop and WebView main thread, generation-aware explicit cancellation, explicit refresh completion, a backpressured receive queue, incremental snapshots with an explicit completion flag, a bounded stderr tail for attaching recent process output to an error, full stderr logging, request-correlated configuration application results, and orderly shutdown. Root candidates carry a required title and optional presentation subtitle; the subtitle does not affect identity or ranking unless the extension also supplies it as an alias. `invoke` identifies both the selected entry and action. Requests remain pending until their correlated completion, explicit cancellation, protocol closure, or process exit. `configurationChanged` always carries a complete host-validated snapshot. `configurationApplied` means the extension has completed the configuration-dependent work it chooses to include in application; it never means only that the request entered a queue. A concrete correlated Error is the terminal failure result. Actions and configuration applications are never replayed after an ambiguous crash. Accepted completion messages are stored in queues and are not overwritten by later results. Successful `result` messages commit contextual usage through the storage owner. A stale response from an explicitly superseded query is ignored by request ID and generation; every error frame must identify its request, and an uncorrelated error is itself an explicit protocol failure.

The ACP adapter negotiates stable v1, creates one session, and contributes a prompt candidate only for `@<extension-id> <prompt>`. The host supplies the effective extension configuration under `nanika.configuration` in the standard `session/new` `_meta` map. ACP defines no Nanika live-configuration acknowledgement, so a saved change reports `SavedForNextLaunch` and applies when a later session is created. It streams text outside the Tauri event-loop and WebView main thread. The 8 MiB line limit is an explicit transport validation boundary; the 64 KiB stderr tail affects only how much recent stderr is attached to a process error, while the full stream is logged. Escape or dismissal requests cancellation through ACP and waits for the operation to acknowledge completion or fail. Cancellation does not start a replacement process. Each invocation has a unique host ID. Workers publish protocol-neutral output updates with backpressure, and accepted output is not truncated or discarded. ACP extensions receive no Nanika host-service privilege by default.

Host messages: `initialize`, `query`, `invoke`, `cancel`, `refresh`, `configurationChanged`, `hostResponse`, `error`, and `shutdown`.

Extension messages: `initialized`, `snapshot`, `result`, `refreshed`, `configurationApplied`, `hostRequest`, `error`, and `shutdownAck`.

Each `hostRequest` is bound to its parent invocation and generation, and extensions validate matching `hostResponse` fields. The same router handles built-in and external extensions. Service owners have independent queues and independent initialization failure, so one unavailable service does not disable the others. Queue capacity applies backpressure; accepted work waits and never expires before its side effect. A host-service request remains inside its parent invocation until completion, explicit cancellation, or service closure. Current services accept typed launch descriptors and typed clipboard writes; image writes are confined to the requesting extension's machine-local payload root and are read with explicit encoded and decoded resource limits.

Nanika requests carry IDs; query, action, and refresh messages also carry a generation. ACP uses its standard JSON-RPC request and session IDs, plus a host invocation ID for output correlation. Superseded query generations are cancelled because the latest query is authoritative; side-effecting operations are never discarded or replayed. Initialization and actions have no hidden watchdog deadline. Orderly shutdown asks the extension to stop and waits for acknowledgement. Explicit host termination, startup containment failure, and application shutdown terminate the owned process tree. Windows uses hidden suspended creation and binds a kill-on-close Job Object before the initial thread runs; macOS uses a dedicated process group and propagates termination failures other than an already-missing group.

External packages are ZIP archives with a `.nanika` suffix:

```text
manifest.jsonc
bin/<target>/<entrypoint>
resources/
README.md
LICENSE
```

External extensions use the validated `ExtensionManifest` schema inside `.nanika` packages. Every built-in uses the same complete schema in its own `manifest.jsonc`; the Host-owned inventory selects which validated manifests and companion binaries belong to the distribution. Release packaging must verify that set against the signed artifact. No extension-controlled field can declare built-in identity.

The manifest declares no frontend contribution or WebView entrypoint. `resources` contains inert extension-owned data for the native extension process; the shell never mounts an extension resource directory into the frontend, and the WebView never loads extension HTML, CSS, JavaScript, modules, preload code, or remote UI. A visible static resource requires a dedicated bounded protocol type and host validation before the shared frontend renders it.

Manifest version 1 requires `runtime: { protocol, protocolVersion }`; current values are Nanika v1 and ACP v1. Unknown protocols, versions, fields, targets, unsupported or duplicate permissions, and unsafe entrypoints are rejected. IDs and dependency IDs are lowercase reverse-DNS segments, and package versions use Semantic Versioning. The MVP supports `process.launch` and `clipboard.write`. Extensions declare host integration under `contributes`: static `commands`, static declarative `views`, presence-based dynamic `rootSearch`, and the optional `configuration` schema. Command and View IDs are unique across their extension. ACP extensions retain their prompt activation path and may declare configuration, but not Nanika commands or views. Capabilities, dependencies, and activation events remain reserved.

Contribution declarations are independent and composable:

```json
{
  "contributes": {
    "views": [
      {
        "id": "clipboard.history",
        "title": "Clipboard History",
        "description": "Search and copy previous clipboard content.",
        "icon": "clipboard",
        "keywords": ["clipboard", "history", "paste"]
      }
    ]
  }
}
```

Installation alone contributes no Root Search row. Each declared command or View contributes one static row, while `rootSearch` contributes only the dynamic Candidates returned for the current query. A static command activates with the protocol-defined `command.execute` action; a static View activates with `view.open`. A dynamic Candidate declares its own `action` or `view` kind and action ID. Root Search uses the kind only for generic interaction policy; the extension's correlated result remains authoritative for the resulting navigation effect.

`nanika-cli` installs, updates, enables, disables, and removes external extensions while the host is stopped. `install` creates a missing extension or repairs the same immutable version; a different installed version requires `update`. `update` requires an installed extension, preserves enablement, and rejects downgrades. Archives are limited to 128 MiB, 4,096 entries, and 512 MiB expanded content. Traversal, symlinks, cross-platform name collisions, filesystem collisions, unsupported compression, and excessive compression ratios are rejected. Extraction never overwrites an existing path. The package is copied and hashed once, then extraction uses that immutable staged copy. The target entrypoint is made executable on macOS. One CLI operation uses ordered mutations and synchronous compensation so a reported failure does not knowingly leave mixed configuration, database, and artifact state. An interrupted transaction is detected and reported explicitly on the next operation; it is not completed, rolled back, or deleted silently. Built-in IDs cannot be replaced or removed. No marketplace, development-directory install, or background download service is included.

The temporary `nanika-extension-acp-dummy` is an ordinary workspace extension that implements stable ACP v1 and returns `Hello World`. Rust tests package it as `.nanika`, install and resolve it through the normal external-extension path, publish its explicitly activated prompt candidate, and verify streamed output through the host adapter. Protocol tests verify v1 negotiation, unique session IDs, explicit frame bounds, request correlation, cancellation, orderly shutdown without relaunch, startup containment, and descendant termination. It has no runtime privilege, is excluded from release packaging, and must be removed before 1.0.

## Performance and validation

Targets:

- Warm summon to interactive overlay: P95 at or below 50 ms.
- Text input or navigation event to its next visual update: P95 at or below 16.7 ms.
- Committed query to the first coherent result state: P95 at or below 50 ms.
- Stable 60 FPS, with 120 Hz support when available.
- Hidden idle path: no animation frame loop, frontend polling, or unnecessary event delivery, with near-zero CPU.
- No filesystem, SQLite, or blocking extension work on the Tauri event-loop or WebView main thread.

Measure p50, p95, p99, frame-time variance, long tasks, dropped frames, CPU, memory, process count, database size, and thread count on fixed representative Windows and macOS machines. Benchmark query delivery, startup, indexing, extension activation, persistence, frontend commit, layout, and paint separately. Use `criterion` only for isolated deterministic Rust benchmarks. End-to-end evidence must come from a Rust controller driving the unchanged Nanika application through real extension processes, Tauri IPC, the system WebView, and the production frontend; browser-realm timing uses the native Web Performance API. Performance changes require evidence.

The workspace release profile uses thin LTO and one codegen unit, and `pnpm --dir apps/desktop build` can produce an unsigned Tauri release-mode bundle. Signed archives and the release pipeline are not implemented. The planned Windows artifact is a signed portable x86-64 ZIP; planned macOS artifacts are Developer ID signed, hardened, notarized, and stapled `.app` ZIPs for Apple silicon or Intel. Every published artifact will be immutable, versioned, and paired with SHA-256. The MVP has no installer or updater framework; update and rollback replace the complete stopped application from a verified artifact while preserving external user data.

## Deferred or rejected

- Direct core use of `tokio`, `anyhow`, `log`, and `rayon`.
- Rust dynamic-library extensions, `libloading`, `abi_stable`, `interprocess`, Wasmtime, and WASI for the MVP.
- Extension marketplace, background downloads, cloud sync, generated-data sync, and enforceable sandboxing.
- Draft ACP v2, production agent UX, file search, URL search, and other later capabilities.

### Input errors, cancellation, and admission

The host publishes its query character limit in session metadata. The launcher counts Unicode code points, retains the complete input and previous results when the limit is exceeded, and displays an associated validation message. Invalid input cannot invoke stale results. Editing back within the limit resumes search. Query submission errors do not replace the search surface or require a reload; global failure is reserved for session transport or core startup failure.

Each action receives a unique protocol request ID independent of search generation. Sending Cancel requests cancellation; it does not establish that an action was cancelled. Native protocol cancellation completes with a correlated Error whose code is `cancelled`. A Result that wins a cancellation race remains a successful completion. ACP uses the peer's StopReason rather than inferring cancellation from the local signal. Queued cancelled actions never reach the process. An accepted host service request retains its response receiver through explicit action cancellation and reports its real outcome. Application shutdown remains a separate process-termination operation.

The native adapter drains a superseded query's correlated terminal frame before sending subsequent work. Its terminal error is recorded for diagnostics but cannot fail the next query. Calculator protocol input runs separately from evaluation and supplies an atomic explicit-cancellation signal to fend-core. There is no evaluation deadline.

Each extension admits at most 16 queued actions, view requests, refreshes, and configuration applications in total, in addition to active work. Admission waits for capacity without holding the worker state lock. Worker exit and shutdown wake blocked submitters with an explicit closed result. Shell invocation admission and durable execution recording run on blocking workers after releasing shared shell state; the frontend disables repeated activation while the current invocation is pending. Closing view-result delivery precedes joining extension workers so a full result queue cannot prevent shutdown.
