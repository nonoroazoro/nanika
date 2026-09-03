# Nanika Technical Stack

Status: current pre-1.0 baseline. Tauri is the only desktop UI solution. The Rust core remains authoritative.

## Selected baseline

| Area | Selection | Boundary |
| --- | --- | --- |
| Languages | Rust stable and TypeScript | Rust owns the core and privileged desktop boundary. TypeScript owns frontend presentation and local interaction. |
| Platforms | Windows 10 and macOS 13 or later | Validate both platforms; keep platform code behind adapters. |
| Desktop shell | Latest mutually compatible stable Tauri 2 ecosystem | The Rust shell owns windows, IPC, capability configuration, custom protocols, tray integration, and application lifecycle. Review current Tauri releases and official guidance at every dependency update. |
| Frontend | Latest mutually compatible stable Svelte 5 and TypeScript | One shared local frontend for Windows and macOS. Use plain Svelte as a client-only application with no SvelteKit, server rendering, remote code, or runtime CDN assets. |
| Frontend build | Vite and the official Svelte Vite plugin | Build one local static application into `dist`. Tauri starts the Vite development server and consumes only the production build output in release artifacts. |
| Frontend package manager | pnpm | Pin the pnpm version in the desktop application's package metadata, commit its `pnpm-lock.yaml`, and use exact direct dependency versions. |
| Frontend tool runtime | Current Active LTS Node.js | Pin one development and CI version. Node.js is not an application runtime and must not appear in release artifacts. |
| Frontend static validation | `svelte-check`, ESLint with Svelte support, and Prettier with Svelte support | Validate Svelte templates, component contracts, TypeScript, accessibility diagnostics, code defects, and deterministic formatting before tests and production builds. Treat warnings as failures in CI. |
| UI platform | Semantic HTML and CSS | Use native text editing, DOM focus, browser scrolling, ARIA patterns, CSS custom properties, and system font stacks. |
| Frontend localization | Typed local message catalogs and browser `Intl` | Rust includes the operating-system locale in the initial application snapshot. Select a bundled catalog with deterministic English fallback and format locale-sensitive values without an i18n framework or remote resources. |
| WebView | System Evergreen WebView2 on Windows and WKWebView on macOS | Do not bundle a fixed WebView2 runtime in the portable MVP. Treat engine and operating-system versions as part of the validation matrix, fail clearly when the required runtime is unavailable, and do not depend on unsupported experimental web features. |
| Rust to frontend boundary | Bounded Tauri commands and channels | Commands handle requests and mutations. Channels deliver ordered snapshots and output. Tauri events are reserved for small, low-frequency lifecycle notifications. All payloads use serializable DTOs with contract-tested TypeScript types. |
| Frontend Tauri API | `@tauri-apps/api` | Import only required ESM APIs inside the typed bridge. Keep the global Tauri object disabled and do not add frontend plugin bindings when Rust owns the capability. |
| Frontend security | Tauri Isolation Pattern, capabilities, strict CSP, validated application commands, and validated custom protocols | A dependency-free isolation application filters frontend IPC before Rust. Capabilities grant only required application, core, and plugin commands through explicit permissions. Command implementations validate requests and scopes. Bundle local assets only and deny arbitrary filesystem, process, shell, navigation, and remote network access. |
| Global hotkey | Official `tauri-plugin-global-shortcut` Rust API | One configurable normal modifier-and-key shortcut with identical user-visible semantics on both platforms. The frontend receives no global-shortcut permission. |
| Fuzzy matching | `nucleo-matcher` | One persistent matcher owned by the named search owner thread. |
| Application paths | `directories` | Resolve roots once through `ProjectDirs`. |
| Directory traversal | `walkdir` | Bounded recursive scans without a general parallel walker. |
| Windows discovery | `windows` and `windows-sys` | Typed Shell COM plus direct known-folder, executable, and icon APIs. |
| macOS discovery | `std::fs`, `plist`, `icns`, and `objc2` platform crates | Localized application bundles, `Info.plist`, and normalized native icons. |
| Icon cache encoding | `png` | Deterministic RGBA PNG cache variants and fallback icons. |
| Clipboard | `clipboard-rs` | Native Windows monitoring and measured macOS pasteboard polling. |
| Calculator | `fend-core` | Deterministic, interruptible, arbitrary-precision local evaluation. |
| Startup | `windows-registry` and `objc2-service-management` | Current-user Windows Run entry and macOS `SMAppService.mainAppService`. |
| Tray and menu bar | Tauri `tray` and `menu` Rust APIs | Build and handle the tray entirely in the Rust shell. Expose only Open Nanika, Settings, Rescan applications, and Quit. The frontend receives no tray or menu mutation permission. |
| Single instance | Nanika per-user Windows and macOS activation adapter | A foreground second launch emits an activation event to the Tauri shell; a background launch exits without activation. The official plugin remains unsuitable until it guarantees the same per-user transport and activation contract. |
| Serialization | `serde`, `serde_json`, `jsonc-parser` | JSONC only for human-edited files and manifests. Internal APIs use typed Rust values. |
| Extension IDs and versions | `uuid` and `semver` | UUID v4 for opaque IDs and Semantic Versioning for packages. |
| Database | SQLite through `rusqlite` | Default features disabled; only `bundled`. |
| Background work | Standard-library owner threads in the core | Tauri runtime facilities stay at the shell boundary. No blocking core work runs on the Tauri event-loop or WebView main thread. |
| Process launch | `std::process::Command` behind platform adapters | Structured arguments by default; explicit shell mode only. |
| Errors | `thiserror` and standard error traits | Typed errors at crate and host boundaries. |
| Diagnostics | `tracing`, `tracing-subscriber`, and `tracing-appender` | Non-blocking local logs with duplicate suppression and a hard byte cap; no content-bearing query or clipboard fields. |
| Benchmarks | `criterion` as a dev dependency | Default features disabled; targets stay outside runtime crates. |
| Frontend tests | Vitest, Vitest Browser Mode, `@vitest/browser-playwright`, and `vitest-browser-svelte` | Use a Node project for pure TypeScript and real Chromium and WebKit projects for Svelte components. Test roles, accessible names, keyboard and pointer input, focus, CSS layout, and observable state. Mock Tauri only at the typed bridge. All test packages are development-only. Release acceptance still exercises packaged WebView2 and WKWebView applications. |
| Extension runtime | Host-supervised child processes | Every extension uses the same lifecycle and failure boundary; a versioned schema selects its wire adapter. |
| ACP | Official `agent-client-protocol` SDK with `async-io`, `async-channel`, `async-process`, `futures`, and `futures-lite` | Stable ACP v1 only. One isolated supervisor thread drives each ACP process; `rustix` terminates the macOS process group. No project-wide executor. |
| Extension package | ZIP with `.nanika` suffix | `zip` default features disabled; only `deflate-flate2-zlib-rs`. |
| Package integrity | `sha2` | SHA-256 for corruption detection. |

Use the latest mutually compatible stable releases when adding or updating dependencies. Commit `Cargo.lock`. Do not use Git dependencies, wildcard versions, or pre-release versions by default. Review each non-standard-library dependency for necessity, features, transitive cost, maintenance, and platform support.

Commit the desktop application's `pnpm-lock.yaml`, pin the pnpm version through its package metadata, and pin the current Active LTS Node.js line for development and CI. Use exact direct dependency versions. The initial frontend has no SvelteKit, router, global state framework, component library, utility CSS framework, CSS-in-JS runtime, animation framework, simulated DOM, icon font, analytics SDK, remote font, or remote asset dependency. Add one only after a demonstrated requirement and architecture review. Prefer platform APIs, Svelte-local state, semantic HTML, plain CSS, and static local assets.

The initial production frontend imports only Svelte runtime modules emitted by the compiler and the required ESM surface from `@tauri-apps/api`. Tauri plugin guest packages are absent unless a reviewed feature must be owned by the frontend. Vite, the Svelte Vite plugin, TypeScript, Node.js, pnpm, static-analysis tools, Vitest, browser providers, and test renderers are development-only. Inspect the generated `dist` instead of inferring bundle contents from package metadata.

Use current stable HTML and CSS features supported by the minimum WKWebView baseline and the system Evergreen WebView2 runtime. Prefer CSS custom properties, logical properties, Grid, Flexbox, `color-scheme`, media queries, and feature queries over compatibility libraries or JavaScript layout. A newer platform feature must have an explicit fallback when support differs across the validation matrix.

Tauri serves a local static frontend. The desktop package scripts run from `apps/desktop`; `pnpm frontend:dev` starts the Vite project in `apps/desktop/frontend`, and `pnpm frontend:build` writes its production output to `apps/desktop/frontend/dist`. The shell config in `apps/desktop/shell` sets `frontendDist` to `../frontend/dist` and uses those package scripts for `beforeDevCommand` and `beforeBuildCommand`. Development uses a fixed Vite server configured for Tauri, while production has no application server, server-side rendering, remote entrypoint, or runtime CDN dependency.

## Tauri adoption policy

Use the highest-level current stable Tauri 2 primitive that fully preserves Nanika's product, performance, security, and cross-platform contracts. Prefer Tauri core APIs first, official Tauri plugins second, and a narrow platform adapter only when the stable Tauri surface cannot express a required behavior. Use official plugins from Rust when the frontend does not need their authority, and do not install their JavaScript guest bindings or grant their commands merely for convenience.

The initial Tauri baseline enables these current stable capabilities:

- The Isolation Pattern with a dependency-free classic-script isolation application that allowlists command names and validates coarse payload envelopes before IPC reaches Rust. Rust command permissions, scopes, and validation remain authoritative.
- `build.removeUnusedCommands = true` with standard static capability files, no dynamically added ACLs, and explicit permissions instead of broad default permission sets.
- Tauri commands for request-response work, channels for ordered streaming, and events only for low-frequency Rust-to-frontend lifecycle notification.
- Tauri `WebviewWindow`, `tray`, `menu`, lifecycle, scale-factor, theme, and monitor APIs at the Rust shell boundary.
- Official `tauri-plugin-global-shortcut` through its Rust API, with no frontend permission.
- Tauri managed state for one typed shell handle to the UI-independent Rust services. Do not duplicate domain state in Tauri, use global mutable state, or hold a state lock across blocking work or an await point.
- Platform-specific Tauri configuration files for settings that genuinely differ between Windows and macOS.

Isolation adds cryptographic IPC work. Measure its summon, query, navigation, and channel overhead in release builds, keep the isolation application free of third-party dependencies, and optimize the message shape or frequency if targets are missed. Replacing Isolation with Brownfield requires an explicit security and performance decision, not a silent fallback.

Current narrow exceptions are deliberate. Keep the custom per-user single-instance adapter because Nanika requires foreground-versus-background activation semantics and per-user transport behavior not guaranteed by the official plugin. Keep native startup integration because the current official autostart plugin uses LaunchAgent or AppleScript on macOS rather than `SMAppService`. Keep active-monitor placement, native hotkey timing observation, and process containment behind platform adapters because the higher-level Tauri APIs do not provide their complete contracts.

Do not enable unstable multiwebview support or another experimental Tauri feature in the product baseline. At each Tauri ecosystem update, review release notes, remove obsolete workarounds, adopt newly stable capabilities when they replace custom code without regression, and record any remaining exception in this section.

Stable Tauri native window effects may progressively enhance the launcher through platform-specific configuration. The semantic CSS surface remains complete without them. Select an effect only after physical Windows and macOS validation confirms text contrast, transparent-window startup, compositor cost, resizing, focus transitions, and fallback behavior.

The frontend uses Svelte 5 runes and current component syntax only. Use `$state` for local mutable presentation state, `$derived` for derived state, and `$effect` only for external synchronization that cannot be expressed by an event handler or lifecycle boundary. Do not use legacy reactive statements, legacy component event directives, or Svelte stores for component-local state. Use `<svelte:boundary>` around the application root and extension route content to map render and effect failures into safe diagnostic states. Event-handler and asynchronous failures remain handled explicitly because Svelte boundaries do not catch them.

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
      vitest.config.ts
      src/
      tests/
    shell/
      Cargo.toml
      build.rs
      tauri.conf.json
      tauri.macos.conf.json
      tauri.windows.conf.json
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
  domain/
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
  release/
```

`apps/desktop` is the only desktop application boundary. Its package manifest and lockfile coordinate frontend and Tauri commands, while implementation source remains separated. `apps/desktop/frontend` contains only the browser-realm Svelte presentation layer, frontend assets, and matching tests. `apps/desktop/shell` contains only the privileged Tauri executable, configuration, capabilities, isolation application, bundle resources, and native lifecycle integration. `frontend` and `shell` describe product responsibilities instead of implementation languages. `shell` replaces the scaffold-default `src-tauri` name.

`apps/desktop/shell`, `apps/cli`, and every executable below `apps/extensions` are members of the root Cargo workspace. `apps/extensions/built-in` contains extensions shipped with Nanika. `apps/extensions/fixtures` contains non-shipping executables used to verify the standard extension and ACP boundaries. `engine` members may depend on other `engine` members but never on Tauri, WebView, Svelte, browser, or desktop-shell types. The shell depends inward on `engine`; the frontend communicates only through bounded shell contracts. Extension executables depend on `engine/extension-protocol` and other approved engine libraries but never on the frontend or shell.

The frontend remains one local plain Svelte 5 pnpm project built by Vite. Do not introduce a repository pnpm workspace for a single package. The desktop-level `package.json` is command and dependency metadata, not a second frontend package. Frontend configuration points explicitly at `frontend`; generated assets stay in `apps/desktop/frontend/dist` and remain untracked.

Directory names under `engine` describe responsibilities while Cargo package names remain stable: `nanika-core` lives in `engine/domain`, `nanika-host` lives in `engine/runtime`, `nanika-protocol` lives in `engine/extension-protocol`, `nanika-config` lives in `engine/configuration`, and `nanika-extension-package` lives in `engine/extension-management`. The `platform`, `storage`, and `search` names already describe their responsibilities accurately.

Do not create top-level `crates`, `extensions`, `src-tauri`, `web`, `rust`, `scripts`, `packaging`, or `dist` directories. Cargo's root `target` remains the single generated build tree; release archives and evidence belong under named subdirectories of `target`, not a new repository-root output directory. The runtime contains reusable orchestration and service logic or is divided into smaller UI-independent engine members when ownership becomes clearer.

## Cross-platform architecture

Shared core, frontend, diagnostics, protocol, configuration, storage, search, and extension lifecycle behavior is platform-neutral. The same frontend source and Rust contracts run on Windows and macOS. Platform-specific behavior exists only behind typed adapters in `nanika-platform`, the narrow Tauri shell boundary, or an extension's explicit platform adapter. A shared feature is not complete if it works on only one supported OS.

Every platform adapter contract must preserve the same user-visible semantics, failure boundary, cancellation behavior, and diagnostics shape on Windows and macOS. Platform implementations may use native APIs, but platform details must not leak into shared state or wire protocols. Linux-specific behavior is not an acceptable fallback and must not enter shared paths unless Linux becomes an explicit supported target through a baseline update.

## Host and extension boundary

The Rust host foundation provides scheduling, persistence boundaries, diagnostics, permissions, platform services, extension lifecycle, search orchestration, and shared interaction state. The Tauri shell provides desktop lifecycle and the privileged frontend boundary. The frontend provides presentation and local interaction. None of these layers contributes application, command, script, calculator, clipboard history, or agent capability.

Every domain capability is an extension. This follows the relevant VS Code model. There is no first-party capability class.

- `Built-in`: an extension executable shipped with the default Nanika distribution and enabled by default. It cannot be uninstalled because it belongs to that distribution.
- `External`: an extension executable installed from a `.nanika` package.

Both forms use the same capability contract, lifecycle, settings contribution, permissions, host services, process supervisor, and failure policy. Built-in status grants no extra privilege. The bare host has no domain capability. The default distribution enables command, application, script, calculator, and clipboard history extensions.

A development launch builds the Tauri desktop application and all built-in extension packages. The packaged Rust shell resolves companion executables from validated bundled resource locations. Building or copying only the shell is an invalid development or packaging layout and produces feature-specific startup diagnostics.

### Process boundary

Every extension runs as a separate host-supervised child process. The host owns process creation, protocol I/O, cancellation, timeout, restart, shutdown, reaping, and resource budgets. On Windows, the host creates extension processes suspended, assigns their kill-on-close Job Object, and resumes them only after containment succeeds. On macOS, each extension starts in its own process group. Host APIs never expose host memory, SQLite connections, global configuration, or another extension's state. Built-in packaging never bypasses this boundary. The MVP does not provide an enforceable OS sandbox, so a child process retains the filesystem access of the current user.

Built-in extension executables are declared through Tauri `bundle.externalBin`. Build inputs use Tauri's required `-$TARGET_TRIPLE` filename suffix, while release inventory verifies the final platform bundle names and locations. The Rust process supervisor resolves and starts only validated bundled executables and retains the containment behavior above. The frontend receives no Tauri shell or process permission, and the Tauri shell plugin is not part of the frontend execution path.

Do not load extensions in-process or through Rust dynamic libraries. This is process and failure isolation, not a security sandbox. MVP extensions are trusted native code; enforceable isolation requires a future sandbox decision.

### Shared interaction

The product owns Root Search. Rust owns input history, search aggregation, contextual ranking, final ordering, execution, and durable state. The frontend owns the focused text field, active option, local keyboard interaction, and scroll presentation. Extensions may contribute static commands and bounded dynamic candidates. They do not control cross-extension ordering.

A command may complete without a view or push a route-local declarative view. The extension supplies a bounded `ListView` or `DetailView`; the shared frontend owns pixels, typography, accessibility, keyboard behavior, focus, and platform consistency. A list may request the semantic `Plain` or `Split` layout, sections, selection, detail content, filters, pagination, and typed item actions. A standalone detail may declare actions; actions for a detail nested in a list belong to its selected list item. The extension never receives HTML, CSS, JavaScript, a DOM reference, a WebView handle, a native handle, or an arbitrary drawing surface.

Each pushed view has an extension-scoped ID and monotonic revision. Rust validates every view document and serializes extension operations. The frontend applies only matching revisions, keeps local text editing and selection synchronous, and coalesces outbound search and selection updates. Back closes the active frontend route immediately. Overlay dismissal closes every extension route in reverse stack order through one bounded command. Nested routes are bounded, and stale updates cannot mutate a different route.

`ViewActionStyle` communicates primary, secondary, or destructive prominence. It does not grant behavior or permission. Every action is rendered by the shared frontend and returned through a typed Tauri command to the owning extension. Host services such as clipboard writes and process launches remain separately permission checked in Rust.

## UI and interaction

Use one undecorated, transparent, always-on-top Tauri `WebviewWindow` for the launcher and a separate decorated window for Settings. macOS transparency requires `app.macOSPrivateApi`; enabling it excludes Mac App Store distribution, which is acceptable only while the supported macOS artifact remains a directly distributed, Developer ID signed and notarized application. Set `bundle.macOS.minimumSystemVersion` to `13.0` instead of accepting Tauri's lower default. On Windows, keep the launcher native shadow disabled unless physical validation proves it introduces no border artifact. Evaluate `noRedirectionBitmap` only if measurements reproduce a transparent-window startup flash and confirm the option fixes it without regressions.

The primary process starts hidden with only its tray or menu-bar visible. The launcher frontend is loaded and ready before the first summon. The shell shows it only after current state is available, then requests frontend focus and waits for an interactive acknowledgement. The shell owns visibility, placement, native focus, scale-factor changes, and application lifecycle. Tauri window size and position use logical pixels; platform adapters convert physical monitor geometry with the current scale factor before placement and re-evaluate it when the monitor or scale factor changes. The WebView owns text input, IME, DOM focus, layout, painting, and accessibility.

The shared visibility contract is `show`, `hide`, and `toggle`. The frontend never decides native visibility. Platform-specific focus, active-monitor placement, full-screen behavior, elevated-window behavior, and any required macOS ordering workaround stay behind the Tauri shell and `nanika-platform`. Hidden state must stop animations, timers, observers that poll, and unnecessary event delivery.

The frontend owns the visual language through plain CSS, semantic CSS custom properties, and reusable Svelte components. Global styles define reset, tokens, platform themes, and shared primitives; component styles remain scoped and consume semantic tokens without private product-specific color or spacing systems. Root Search and extension views share the same SearchInput, ResultList, ResultRow, SectionHeader, ActionBar, KeyHint, DetailPanel, and state components. Svelte's built-in transitions and motion primitives implement state-driven motion only where CSS alone cannot express the lifecycle.

The visual baseline is compact, neutral, content-first, and platform-aware without maintaining separate designs. Use the operating-system UI font stack through CSS. Do not bundle a font unless platform testing proves a missing glyph or metric defect that cannot be solved by the system stack. Text inputs use semantic HTML controls with native selection, caret, IME, and accessibility behavior. CSS controls size and spacing without replacing text editing.

Root Search uses the ARIA combobox pattern with a listbox popup. DOM focus remains in the search input. Unmodified Up and Down change `aria-activedescendant`; Ctrl+Up and Ctrl+Down navigate input history. Enter invokes the active option. Escape dismisses the launcher. Reopening selects a non-empty query. The active option is revealed with the browser scroll container only when its bounding rectangle crosses the scrollport boundary. Navigation clamps at the first and last option.

Animations use CSS transitions or the Web Animations API only when state-driven motion needs interruption. Every motion defines duration, easing, interruption, and reduced-motion behavior. Hidden UI has no active animation frame loop. Summon must never expose an empty document or stale view before interactive readiness.

The frontend renders at the WebView device pixel ratio and uses vector CSS or sufficiently large raster assets. Application icons are served through a validated custom protocol by opaque identity. The frontend cannot construct filesystem paths. The protocol accepts only the required read method, binds requests to an authorized window label, serves only completed cache variants as immutable content with an explicit MIME type, and rejects unknown identities, traversal, incomplete retry entries, and out-of-scope roots. Tauri's built-in asset protocol remains disabled because Nanika does not expose a general filesystem-backed asset surface.

The MVP includes a minimal desktop-shell-owned tray or menu-bar item:

- Windows notification-area tray icon.
- macOS `NSStatusItem`.
- `Open Nanika`, `Settings`, `Rescan applications`, and `Quit`.

The Settings view contains host settings and dynamically contributed settings from every enabled extension. Built-in and external extensions use the same settings schema and validation path. JSONC remains available as an advanced editing path.

Settings use a bounded declarative contract with toggle, text, string-list, and record-table controls. The frontend renders controls but does not interpret domain configuration. Each extension has one editable contribution and at most one request-correlated update in flight. Its draft is locked until Rust validation and atomic JSONC persistence complete. Host controls remain read-only until their runtime owners load. Application path lists and script records use this same contract; extensions without configurable values contribute an empty section. Host settings contain the hotkey and reduced-motion preference. Startup state remains OS-owned.

Rust resolves the operating-system locale at startup and includes its normalized language tag in the initial application snapshot. The frontend selects from bundled typed message catalogs, falls back deterministically to English, and uses browser `Intl` for supported locale-sensitive formatting. Application display names remain extension-provided localized values with original-name aliases. No runtime translation download, localization framework, or frontend OS-information permission is included.

## Tauri application boundary

Rust is authoritative for domain state and side effects. The frontend is authoritative only for ephemeral presentation state such as input composition, active option, scroll position, open menus, and interrupted visual transitions.

Frontend-to-Rust commands are narrow and task-oriented:

- Open an authorized frontend session with a bounded state channel and return its initial application snapshot without a subscription race.
- Publish the latest committed query through a coalesced slot.
- Invoke a candidate by stable extension, entry, and action identity.
- Submit a typed extension view event with view ID and revision.
- Read and update typed settings contributions.
- Request rescan, diagnostics export, route close, launcher dismissal, or application shutdown.
- Acknowledge frontend readiness and focused interactive state for activation tracing.

Rust-to-frontend channel messages carry current state, not imperative drawing instructions:

- Root Search snapshots with generation and completion state.
- Extension route snapshots with view ID and revision.
- Invocation output deltas with invocation ID and bounded visible content.
- Runtime capability state and safe user-visible diagnostics.
- Launcher state needed to reset or preserve ephemeral frontend state.

The session command registers its channel before capturing the initial snapshot. Every asynchronous payload carries the identity needed to reject stale work. Channel delivery is bounded; slow or disconnected consumers cannot block a core owner thread. Commands never return borrowed core state, platform handles, filesystem paths, process descriptors, or unbounded extension payloads. The frontend never infers permission from presentation metadata.

High-frequency query and selection updates use latest-value coalescing. Actions, settings mutations, and other side effects are request-correlated and never replayed after an ambiguous failure. Closing the launcher cancels or detaches work according to the Rust lifecycle contract rather than leaving frontend promises as owners.

Tauri capabilities are window- and webview-specific and grant only the application, core, plugin, and event API permissions each surface uses. Application commands define explicit permissions and validate request scopes in their implementation. Event names, event payloads, and individual channel messages do not provide a per-message authorization boundary. An authorized session command therefore creates each bounded channel, binds it to the invoking `WebviewWindow` label, and sends only data allowed for that session. Production builds disable developer tools, remote navigation, arbitrary URL opening, drag-and-drop file access, the shell plugin, process APIs, the global Tauri object, and the built-in asset protocol unless an approved feature requires them.

Tauri events carry only small, low-frequency, non-sensitive Rust-to-frontend lifecycle notifications where multi-consumer delivery is useful. A consuming surface receives listen permission only; frontend-to-Rust work uses commands and the frontend receives no event emit permission. Search snapshots, extension views, settings state, diagnostics, and invocation output never use the event system. Do not evaluate generated JavaScript to transfer application state.

Configure the production Content Security Policy explicitly. It permits only bundled frontend resources, the minimum Tauri IPC transport required by the generated application, and `nanika-icon` image responses. Let Tauri inject the hashes and nonces required by bundled assets; do not allow remote origins, `unsafe-inline` or `unsafe-eval` for scripts, or broad filesystem-backed asset sources. Development-only allowances must not enter the production configuration.

## Search and ranking

The named search owner thread owns a persistent `nucleo-matcher` instance, aggregation, usage state, final ranking, and generation-tagged snapshots. The Tauri command boundary replaces a coalesced latest-query slot and wakes the bounded owner queue, so saturation cannot drop the current input. Each extension has a fixed protocol worker with a latest-query slot. A new generation waits for the initial snapshot from every ready extension before publishing, which prevents the result list from flashing through partial extension states. Later incremental snapshots replace one coalesced pending channel message, and stale request IDs and generations are discarded.

The `nanika-search` crate implements Unicode lowercase, punctuation-separated, whitespace-collapsed exact, prefix, token, fuzzy, empty-query, and alias matching through `nucleo-matcher`. Extensions may supply localized names as aliases. Candidate search values are normalized once when snapshots enter the host. Queries are capped at 4,096 Unicode scalar values. Fuzzy results require 12 score points per normalized query character. Contextual frequency and seven-day recency decay apply only inside the same lexical tier. Ranking uses top-k selection before sorting; snapshots contain at most 100 results. Each extension contribution is capped at 5,000 candidates and deduplicated by extension, entry, and action identity.

Clipboard history contributes one static `Clipboard History` command to Root Search and does not publish retained clipboard entries there. Invoking the command pushes its route-local `ListView` with a split detail pane, bounded local filtering, content-type filtering, selection, pagination, and typed actions. Clipboard content remains local and never enters diagnostics.

Ranking order is deterministic:

1. Exact match.
2. Prefix match.
3. Token-prefix match.
4. Fuzzy match above the relevance cutoff.
5. Within a tier, bounded query-contextual frequency and recency boosts.
6. Fuzzy score, alphabetical title, and stable identity tie-breakers.

Global popularity cannot outrank a better lexical tier. Usage identity is `(extension_id, entry_id, action_id, query_context)`. History and usage keys lowercase and trim input while preserving punctuation significant to commands and scripts. Input history preserves the current draft and remains bounded. The storage owner is authoritative for usage: it commits first, then updates in-memory ranking. Usage is local-only, resettable, retained for 180 days, and capped at the 10,000 most recent contexts. No SQLite work occurs in the summon or per-keystroke path.

## Paths, configuration, and generated data

Use `ProjectDirs::from("com", "nanika", "nanika")` as the current identity. It produces the macOS bundle identifier `com.nanika.nanika`. This is a reasonable pre-1.0 default and may change if implementation evidence warrants it.

`data_local_dir()` and `cache_dir()` are API methods, not literal directory names:

| Root | Windows | macOS |
| --- | --- | --- |
| `<app-data-root>` | `%LOCALAPPDATA%\nanika\nanika\data` | `~/Library/Application Support/com.nanika.nanika` |
| `<cache-root>` | `%LOCALAPPDATA%\nanika\nanika\cache` | `~/Library/Caches/com.nanika.nanika` |

The local layout is:

```text
<app-data-root>/
  bootstrap.jsonc
  profile/
  databases/
    nanika.db
    extensions/<extension-id>.db
  extensions/<extension-id>/<version>/
  backups/config/
  backups/databases/
  logs/
  payloads/<extension-id>/
<cache-root>/
  icons/<extension-id>/<icon-key>/{32,64,128}.png
  metadata/
```

`bootstrap.jsonc` contains only the effective config-root locator and machine ID. User configuration is under `<effective-config-root>`:

```text
<effective-config-root>/
  nanika.jsonc
  extensions.jsonc
  extensions/<extension-id>/
  machines/<machine-id>/
```

Only this tree is intended for Dropbox or another file-level sync service. Databases, indexes, clipboard content, extension artifacts, logs, backups, and caches are machine-local generated data and are never synchronized. Never synchronize live SQLite files.

Cleanup follows data ownership. A complete application scan prunes unreferenced icon cache entries and reports cleanup failures through the refresh boundary. The application index runs `quick_check` at startup and rebuilds once when open, load, or scan detects SQLite corruption. Incompatible schemas and access failures remain errors. Clipboard retention removes unreferenced managed payloads. Recovery never deletes `nanika.db`, clipboard history, configuration, installed extension artifacts, or logs.

Use JSONC for human-edited configuration and manifests. Parse through `serde` and `jsonc-parser`, keep CST types private, and convert to typed Rust values at the boundary. UI edits use targeted CST changes, preserve comments and formatting, reparse and validate, then replace files atomically. Each settings file has a `formatVersion`. Before the first release, format changes rewrite the baseline. Released formats require ordered migrations, and a failed migration must leave the original file untouched and start read-only.

The current `nanika-config` boundary implements bootstrap creation, absolute relocatable config roots, typed JSONC parsing, comment-preserving top-level and nested-object changes, atomic replacement, path-preserving backups, bootstrap recovery, and read-only fallback. The extension registry selects defaults only for an explicit missing file; other metadata and access failures remain errors.

## Diagnostics

The host uses stable diagnostic codes and categories. `HostDiagnostic` keeps a safe user message separate from a cloneable technical source chain retained by Rust application state. Operational logs contain only the code, category, operation, and explicitly safe context such as a validated extension ID. Independent extension failures carry distinct safe contexts so duplicate suppression does not merge failures from different extensions. Raw worker errors never cross the frontend boundary. Debug formatting redacts messages and sources. Query text, clipboard content, settings values, extension payloads, and external error text are never logged.

User-visible diagnostics describe the unavailable capability and a concrete recovery action without exposing internal extension, process, protocol, or storage terminology. Identical user messages are rendered once even when several independent technical failures share the same recovery path. The underlying diagnostics remain separate in memory and in operational logs.

The primary host is the only log owner. It writes non-blocking INFO-and-higher lifecycle and failure events under `<app-data-root>/logs`. The queue is lossy and bounded to 256 lines. Identical code, operation, context, and level combinations are recorded at most once per 30 seconds, with at most 256 active keys. Startup removes the oldest owned logs above 32 MiB, and the writer stops at the remaining byte budget. Logs rotate daily, retain at most eight files, and flush during orderly shutdown.

`nanika-cli diagnostics <output.zip>` exports version and platform metadata plus at most eight Nanika-owned daily log files and 32 MiB. It rejects symlinks and unrelated files, enforces the byte limit while copying, publishes through a same-directory atomic no-clobber hard link, and can run while the host is active.

## SQLite storage

Use one host database and one database per extension. Every database has exactly one writer that owns its connection and transactions. The host storage owner owns `nanika.db`; each extension owns its database through a named owner thread in its process. Connections never cross process boundaries. Extensions resolve their database path from the host-supplied data root and own their schema and migration definitions.

`nanika.db` baseline tables:

- `schema_migrations(version, applied_at)`
- `extensions(extension_id, kind, installed_version, active_version, install_path, package_digest, state, health, last_error, updated_at)`
- `input_history(id, normalized_query, display_query, use_count, first_used_at, last_used_at)`
- `usage_stats(extension_id, entry_id, action_id, query_context, execution_count, last_executed_at)`

The host, application, and clipboard schemas each start from the current baseline version 1. Host usage identity includes extension, entry, action, and query context, with an index for retention cleanup. Startup parses and validates extension metadata one row at a time, reports malformed rows, and continues with valid extensions.

Application extension baseline tables:

- `schema_migrations(version, applied_at)`
- `scan_state(id, generation, status, started_at, completed_at, last_error)`
- `app_entries(entry_id, source_key, display_name, normalized_name, normalized_tokens, launch_kind, target_path, working_directory, arguments_json, bundle_id, icon_key, file_identity, last_seen_at, stale)`

Clipboard extension owns its content, hash, timestamp, pin, retention, and payload fields. Calculator is stateless in the MVP.

The clipboard schema stores typed text, file-list, or image references with content hashes, byte size, capture and last-use timestamps, and pin state. Unpinned history is retained for 30 days and capped at 500 entries. Image PNG payloads live under `<app-data-root>/payloads/com.nanika.clipboard`, outside synchronized configuration and SQLite.

Every database uses an embedded baseline and an ordered forward-only migration runner. Before the first release, schema changes rewrite version 1 and old development databases must be reset. Released schemas increment versions transactionally; the host rejects newer or non-contiguous histories. Enable `foreign_keys=ON`, `journal_mode=WAL`, `synchronous=NORMAL`, and `busy_timeout=100 ms`. Before a post-release destructive migration, checkpoint WAL and create a consistent snapshot with `VACUUM INTO`. A failed extension migration disables only that extension and never deletes old data.

## Threads and process execution

Use named owner threads for storage, application discovery, search aggregation, and platform event sources. Runtime configuration and storage initialization stay off the Tauri event-loop and WebView main thread. The search owner reuses one `nucleo-matcher` instance. Do not create a thread per query, action, or database operation. Fixed extension workers publish typed snapshots and carry generation IDs. Each active ACP extension owns one additional named supervisor thread for its isolated async protocol executor. Shutdown stops extension workers, storage, search, and platform events in that order.

Only the host process launcher and extension supervisor may create child processes. Extensions submit typed `Program`, `Shell`, or `MacApplication` descriptors. `Program` keeps structured arguments separate, with an explicit Windows raw-argument representation for Shell Links. `Shell` selects `cmd.exe` on Windows and `/bin/zsh` on macOS. MVP launches are detached with null stdio, and action success means the process was accepted. One bounded launcher owner serializes spawn work. Windows releases detached process handles; macOS reaps children through `kqueue` `NOTE_EXIT` events without polling. Captured execution and launched-action process-tree cancellation remain a later descriptor mode.

## Platform adapters

### Single instance

Nanika runs one host instance per user session. Windows uses `Local\com.nanika.nanika` through `CreateMutexW`; a blocking platform event thread owns only the hidden activation window. macOS holds `nanika.instance.lock` with `flock`; a blocking platform event thread owns a local Unix datagram socket under `<app-data-root>`. One-byte activation and stop datagrams cannot leave the listener blocked on a partial stream connection. Both adapters feed bounded platform events to the Tauri shell and tolerate the primary's startup handoff race. A foreground second launch requests activation, then exits. A background second launch exits without activation.

### Global hotkey

The desktop shell owns registration through the official `tauri-plugin-global-shortcut` Rust API and forwards typed events into the Rust application service. The default shortcut is `Ctrl+Space` on macOS and `Alt+Space` on Windows. A press toggles the launcher and ensures the launcher opens for second-instance activation. A press during dismissal interrupts the transition and reveals the existing surface immediately. Keep media keys outside the MVP. Registration conflicts and failed replacement must preserve the previous working shortcut and produce diagnostics.

Measure native hotkey delivery through a passive `nanika-platform` observer before the plugin's `global-hotkey` backend discards the source timestamp. The observer must always continue native event propagation and must never become an alternate hotkey delivery path. Use Carbon `EventTime` on macOS and `MSG.time` on Windows, then expose only a platform-neutral `Duration` to the host. Missing native timing must mark a sample incomplete instead of silently treating callback time as input time.

### Application discovery

The application extension scans standard platform roots and user-configured roots with `walkdir`. Do not follow symlinks by default. Refresh at startup and on explicit rescan only. Persist generated metadata in the application extension database. Keep filesystem access out of the search hot path.

Windows uses known folders and native `IShellLinkW` resolution. macOS scans `.app` bundles, reads `Info.plist`, and resolves the display name against the operating system's preferred languages, including `InfoPlist.loctable`. The localized display name is the title while bundle names and file names remain searchable aliases. Paths are refreshable metadata; bundle IDs and resolved executable identities provide stable identities.

The application extension runs as its own process with one discovery owner. The host registers it through the universal worker path; explicit refresh is cancellable and stays off the Tauri event-loop and WebView main thread. Settings require `formatVersion`; only a missing file selects defaults, while read failures preserve the existing index.

Windows discovery resolves every `.lnk` through Shell COM and validates PE targets before indexing. Validation is reused while canonical path, size, and modification time remain unchanged; benchmarks separate cold validation from warm refresh. Identity uses the canonical executable, effective working directory, and typed arguments, so equivalent shortcuts and direct executables deduplicate without merging different launch behavior. Complete scans stale missing entries and remove entries already stale; cancelled, failed, or partial scans preserve unseen data. SQLite commits each generation atomically. Snapshots below 5,000 entries remain complete; larger indexes use query-aware top-k preselection before host ranking.

Searchable entries publish before icon extraction. The recoverable icon cache uses extension-scoped high-resolution metadata keys, retry markers, alpha-bound visual normalization, exact 32 px, 64 px, and 128 px PNG variants, legacy Windows alpha recovery, and a generated fallback. Normalization crops transparent bounds, scales visible content to a common occupancy, and centers it on a transparent square. macOS first decodes `.icns` and falls back to the native `NSWorkspace` icon when bundle icon metadata is absent or unusable. A candidate publishes the generated fallback identity until every required variant for its content identity is complete. Retry markers and staged variants are never protocol-visible. Successful extraction atomically completes the content identity and publishes a new candidate snapshot, so bytes under a visible identity never change. The frontend requests the 128 px variant for visible rows through `nanika-icon`, lets the WebView decode and downsample it, and relies on HTTP-style immutable caching. No icon filesystem access or extraction runs on the Tauri event-loop or WebView main thread. Complete scans prune unreferenced cache entries; extraction failures increase scan warnings and prune failures fail the refresh. SQLite corruption or a non-database file rebuilds only the derived application index, with one retry; incompatible schemas and access failures remain visible errors. Optional macOS roots do not make a scan partial; bundle executables require executable permissions. Application actions submit persisted typed launch metadata to the common host service. Full Windows and macOS release acceptance remains required.

### Clipboard

The clipboard extension captures permitted text, file lists, and images. Windows uses native change delivery. macOS polls `NSPasteboard.changeCount` through `clipboard-rs` at a 250 ms interval. The watcher only sends bounded events; one owner performs capture, deduplication, retention, payload cleanup, and SQLite persistence. Oversized content is skipped, never truncated: text and encoded file lists are limited to 1 MiB, file lists to 256 paths, and PNG images to 16 MiB, 8,192 pixels per dimension, and 16,777,216 pixels. Explicit refresh completes only after capture and persistence; worker errors are reported through the protocol. The implemented action uses the common host clipboard service, is labeled `Copy to Clipboard`, and closes the view after a successful copy. TODO: define a separate platform-neutral paste-to-foreground host service with Windows and macOS adapters before presenting paste behavior. Clipboard content never enters diagnostics or synchronized configuration.

### Calculator

The calculator extension uses `fend-core` with `evaluate_preview_with_interrupt`. It declines plain search text and standalone values before evaluation, contributing only when the query contains an explicit symbolic or word operator. Evaluation runs in its extension process with a 4,096-character input limit and a 50 ms interrupt deadline. The MVP context is deterministic and stateless. Successful results copy through the common host clipboard service.

### Command and script

The command extension contributes only for queries beginning with `>` and submits the remaining text as an explicit `Shell` descriptor. The script extension loads stable entries from `extensions/com.nanika.script/settings.jsonc`; every entry names an absolute interpreter, script path, structured arguments, and optional working directory. A missing script settings file means an empty contribution. Neither extension creates child processes directly.

### Startup

Windows uses a quoted absolute executable path under the current-user `Run` key. macOS uses `SMAppService.mainAppService` with a minimum supported version of macOS 13. Startup launches Nanika hidden and idle. The operating system remains the source of effective registration state.

Startup status and mutations run through a bounded platform owner and report their effective state back to the host. Windows treats an unexpected existing Run value as needing repair. macOS preserves `RequiresApproval` and `NotFound` instead of collapsing them into a Boolean; approval opens Login Items rather than repeating registration.

The Tauri shell creates the tray and menu with `tauri::tray::TrayIconBuilder` and `tauri::menu` on both platforms. Rust handlers emit only typed `Open Nanika`, `Settings`, `Rescan applications`, and `Quit` application events. Tray behavior never crosses into frontend authority.

## Extension protocol and package

Nanika protocol v1 uses stdin and stdout with a 4-byte little-endian length prefix, an 8 MiB maximum frame, and a UTF-8 JSON object. ACP v1 uses its standard newline-delimited JSON-RPC 2.0 stdio transport with an 8 MiB frame limit in both directions. The two wire protocols never share a stream.

`ExtensionRuntime` is the common supervisor entry for built-in and external extensions. It selects the wire adapter from validated runtime metadata without changing permissions, lifecycle, or failure policy. Invocation outcomes are `Completed`, `Cancelled`, or `Failed`; only completion records usage, and cancellation is not shown as a failure. Failure recovery uses a fixed restart budget. User cancellation may relaunch a non-cooperative extension without consuming that budget; shutdown cancellation never relaunches it.

The Nanika adapter provides typed frames, a registration handshake outside the Tauri event-loop and WebView main thread, generation-aware cancellation, explicit refresh completion, a one-frame receive queue, query, action, and settings deadlines, incremental snapshots with an explicit completion flag, bounded stderr capture, restart budgets, automatic process recovery, request-correlated extension settings results, and orderly shutdown. Root candidates carry a required title and optional presentation subtitle; the subtitle does not affect identity or ranking unless the extension also supplies it as an alias. `invoke` identifies both the selected entry and action. Interrupted queries are safe to retry after restart. Actions and settings updates are never replayed after an ambiguous crash. Outstanding actions are bounded to the result queue capacity, so accepted completion messages are not dropped. Successful `result` messages commit contextual usage through the storage owner. Late frames are ignored by request ID and generation.

The ACP adapter negotiates stable v1, creates one session, and contributes a prompt candidate only for `@<extension-id> <prompt>`. It streams text outside the Tauri event-loop and WebView main thread, limits stderr to 64 KiB and prompt output to 256 KiB, and uses the common handshake and action deadlines. Escape or dismissal cancels the active invocation. Cancellation first sends the ACP notification; timeout or non-cooperation terminates the process tree. User cancellation relaunches the extension without consuming its failure budget, while shutdown does not relaunch it. Each invocation has a unique host ID. Workers publish bounded protocol-neutral delta batches. Rust emits only current output, and the frontend lays out at most the latest 16 KiB. ACP extensions contribute empty settings and receive no ACP client capabilities or Nanika host-service privilege by default.

Host messages: `initialize`, `query`, `invoke`, `cancel`, `refresh`, `getSettings`, `updateSettings`, `hostResponse`, `error`, and `shutdown`.

Extension messages: `initialized`, bounded `snapshot`, `result`, `refreshed`, `settings`, `settingsUpdated`, `hostRequest`, `error`, and `shutdownAck`.

Each `hostRequest` is bound to its parent invocation and generation, and extensions validate matching `hostResponse` fields. The same router handles built-in and external extensions. Service owners have independent bounded queues and independent initialization failure, so one unavailable service does not disable the others. Host service waits remain inside the parent action deadline, and queued work expires before starting a side effect. Current services accept typed launch descriptors and typed clipboard writes; image writes are confined to the requesting extension's machine-local payload root and are read with encoded and decoded resource limits.

Nanika requests carry IDs; query, action, and refresh messages also carry a generation. ACP uses its standard JSON-RPC request and session IDs, plus a host invocation ID for output correlation. The host drops stale generations, applies a 2-second handshake deadline, bounds action time and stderr, performs orderly normal shutdown, and force-terminates a process tree after deadline or failed cancellation. Windows uses hidden suspended creation and binds a kill-on-close Job Object before the initial thread runs; macOS uses a dedicated process group and propagates termination failures other than an already-missing group.

External packages are ZIP archives with a `.nanika` suffix:

```text
manifest.jsonc
bin/<target>/<entrypoint>
resources/
README.md
LICENSE
```

Manifest version 1 requires `runtime: { protocol, protocolVersion }`; current values are Nanika v1 and ACP v1. Unknown protocols, versions, fields, targets, unsupported or duplicate permissions, and unsafe entrypoints are rejected. IDs and dependency IDs are lowercase reverse-DNS segments, and package versions use Semantic Versioning. The MVP supports `process.launch` and `clipboard.write`. Nanika extensions may declare bounded static commands and Root Search participation through `contributions`; ACP extensions retain their existing prompt activation path. Capabilities, dependencies, and activation events remain reserved.

`nanika-cli` installs, updates, enables, disables, and removes external extensions while the host is stopped. `install` creates a missing extension or repairs the same immutable version; a different installed version requires `update`. `update` requires an installed extension, preserves enablement, and rejects downgrades. Archives are limited to 128 MiB, 4,096 entries, and 512 MiB expanded content. Traversal, symlinks, cross-platform name collisions, filesystem collisions, unsupported compression, and excessive compression ratios are rejected. Extraction never overwrites an existing path. The package is copied and hashed once, then extraction uses that immutable staged copy. The target entrypoint is made executable on macOS. Destructive artifact mutations write a recovery journal before rename; host startup and later CLI operations finish or roll back an interrupted replacement or removal. Configuration, database state, and artifacts use ordered mutations with compensation, and generated cleanup is best-effort after logical commit. Built-in IDs cannot be replaced or removed. No marketplace, development-directory install, or background download service is included.

The temporary `nanika-extension-acp-dummy` is an ordinary workspace extension that implements stable ACP v1 and returns `Hello World`. Tests package it as `.nanika`, install and resolve it through the normal external-extension path, publish its explicitly activated prompt candidate, and verify streamed output through the host adapter. Protocol tests also verify v1 negotiation, unique session IDs, bounded frames and deadlines, repeated cancellation, shutdown without relaunch, reusable recovery, startup containment, and descendant termination. It has no runtime privilege, is excluded from release packaging, and must be removed before 1.0.

## Performance and validation

Targets:

- Warm summon to interactive overlay: P95 at or below 50 ms.
- Text input or navigation event to its next visual update: P95 at or below 16.7 ms.
- Committed query to the first coherent result state: P95 at or below 50 ms.
- Stable 60 FPS, with 120 Hz support when available.
- Hidden idle path: no animation frame loop, frontend polling, or unnecessary event delivery, with near-zero CPU.
- No filesystem, SQLite, or blocking extension work on the Tauri event-loop or WebView main thread.

Measure p50, p95, p99, frame-time variance, long tasks, dropped frames, CPU, memory, process count, database size, and thread count on fixed representative Windows and macOS machines. Benchmark query delivery, startup, indexing, extension activation, persistence, frontend commit, layout, and paint separately. Use `criterion` for deterministic Rust benchmarks and platform plus WebView measurements for launch, focus, and frame pacing. Performance changes require evidence.

Release builds use thin LTO and one codegen unit. Windows ships a signed portable x86-64 ZIP. macOS ships a Developer ID signed, hardened, notarized, and stapled `.app` ZIP for Apple silicon or Intel. Every artifact is immutable, versioned, and paired with SHA-256. The MVP has no installer or updater framework; update and rollback replace the complete stopped application from a verified artifact while preserving external user data.

## Deferred or rejected

- Direct core use of `tokio`, `anyhow`, `log`, and `rayon`.
- Rust dynamic-library extensions, `libloading`, `abi_stable`, `interprocess`, Wasmtime, and WASI for the MVP.
- Extension marketplace, background downloads, cloud sync, generated-data sync, and enforceable sandboxing.
- Draft ACP v2, production agent UX, file search, URL search, and other later capabilities.
