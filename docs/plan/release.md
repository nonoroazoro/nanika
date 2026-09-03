# Release Process

Nanika uses immutable versioned artifacts. The MVP has no installer, background updater, or release channel service.

## Artifacts

| Platform | Artifact |
| --- | --- |
| Windows x86-64 | `nanika-<version>-windows-x86_64.zip` |
| macOS Apple silicon | `nanika-<version>-macos-aarch64.zip` |
| macOS Intel | `nanika-<version>-macos-x86_64.zip` |

Every archive has a sibling `.sha256` file. The Windows archive contains the Tauri-built `Nanika.exe`, `nanika-cli.exe`, and the built-in extension executables. It relies on the system Evergreen WebView2 runtime and does not bundle a fixed runtime. The macOS archive contains the Tauri-built `Nanika.app`, the same extension process boundary, and `nanika-cli` under `Contents/MacOS`. Built-in extension executables are declared through `bundle.externalBin`; packaging inputs carry the required target-triple suffix and release inventory verifies the final bundle names and locations.

Only assets and binaries required by the current Tauri application may appear in release artifacts.

## Build and trust

Packaging commands under `tooling/release` must build the production frontend in `apps/desktop/frontend` first, embed only its local generated assets in the Tauri shell, build the Rust workspace and built-in extensions, and then invoke platform packaging. Release archives and intermediate inventories belong under `target`; no generated `dist` directory is allowed at the repository root.

Official Windows builds sign and verify every executable with SHA-256 before archiving. Packaging verifies the system Evergreen WebView2 prerequisite on a clean Windows profile. If the runtime is absent or cannot create the WebView, the Rust shell must present a native recovery diagnostic because the frontend cannot start.

Official macOS builds use the Tauri application bundle target rather than hand-assembling `Nanika.app`. They set `bundle.macOS.minimumSystemVersion` to `13.0`, sign child executables first, sign the Tauri application with hardened runtime and a secure timestamp, verify it, submit it through `notarytool`, staple the application, validate the ticket, and create the final archive. Launcher transparency requires `app.macOSPrivateApi`, so the supported distribution is direct Developer ID distribution and not the Mac App Store.

Tauri versions, stable feature flags, Isolation Pattern assets, command pruning, capabilities, Content Security Policy, custom protocols, official plugins, bundled resources, application identifier, icons, minimum operating-system versions, and the disabled updater configuration are release-controlled and must be reviewed from the packaged artifact.

## Checklist

1. Confirm a clean tree, the intended version, committed root `Cargo.lock` and `apps/desktop/pnpm-lock.yaml`, the pinned pnpm version, and the pinned Active LTS Node.js line.
2. Run Prettier verification, ESLint, `svelte-check`, Vitest Node tests, Vitest Browser Mode component and visual-regression tests in Chromium and WebKit, and the Vite production build.
3. Run Rust workspace formatting, lint, tests, architecture checks, and cross-boundary contract tests.
4. Run deterministic Rust benchmarks and compare them on the same reference machine.
5. Run the Tauri desktop benchmark on fixed Windows and macOS reference machines and retain schema-versioned reports.
6. Complete the platform acceptance list in `performance.md` on physical Windows and macOS machines.
7. Validate every rule in `ui.md` through automated tests, approved captures, or recorded physical platform acceptance.
8. Verify the packaged production Isolation Pattern, command-pruning output, Content Security Policy, Tauri capabilities, custom protocol scope, disabled built-in asset protocol, absence of frontend shell, process, tray, menu, and global-shortcut permissions, absence of WebDriver or test-access plugins, absence of test tooling, development assets, and source maps, bundled asset inventory, recorded JavaScript and CSS sizes, and absence of remote code.
9. Build with release credentials and verify signatures, notarization, archive contents, and SHA-256 files.
10. Extract each archive into a clean user profile and verify first run, summon, settings, actions, diagnostics, removal, WebView runtime availability, and missing-runtime behavior.
11. Confirm the desktop application starts every built-in extension and one failed extension does not prevent other features from loading.
12. Confirm user-visible failures name affected features, provide a recovery action, and do not expose internal process, protocol, path, or storage details.
13. Confirm Root Search and extension inputs support Latin and CJK IME composition, candidate-window placement, aligned text and caret geometry, and query selection after reopen.
14. Confirm Root Search Enter and pointer activation, Up and Down selection, boundary-only scrolling, input history, and stable result publication while typing.
15. Confirm operating-system locale selection, bundled shell translations, deterministic English fallback, locale-sensitive formatting, localized application names, and original-name aliases. Confirm normalized cached icons remain sharp and responsive on standard and high-DPI displays, incomplete icon entries are never served, and fallback-to-complete transitions cannot be trapped by immutable WebView caching.
16. Confirm calculator results appear for explicit symbolic and word operators and do not appear for plain search terms.
17. Confirm extension List, Split, Detail, filter, pagination, nested navigation, Back, scrolling, and every action style through the frontend-rendered declarative protocol path.
18. Confirm Settings preserves typed validation and atomic JSONC persistence.
19. Confirm accessibility roles, active option state, focus order, keyboard operation, contrast, reduced motion, and operating-system text scaling.
20. Confirm hidden idle has no frontend polling or animation frame loop and meets CPU, memory, process, and thread targets.
21. Publish the versioned archives, checksums, and release notes together.

## Update and rollback

Updates replace application files only while Nanika is stopped. User configuration and generated data stay outside the application directory. Retain at least the previous signed artifact and its checksum. Rollback stops Nanika and restores that complete artifact. Never mix desktop shell, core, or built-in extension binaries from different versions.

After the first release, database migrations are forward-only. A release that changes persistent schemas must document its minimum rollback version and backup requirements.
