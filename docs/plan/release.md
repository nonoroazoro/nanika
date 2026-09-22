# Release: Open Work

There is no signed release pipeline yet. Development launch commands and package inputs are defined by the `justfile` and repository manifests; a local debug or unsigned Tauri bundle is not a releasable artifact.

## Artifacts to deliver

- Windows x86-64: signed `nanika-<version>-windows-x86_64.zip`.
- macOS Apple silicon and Intel: Developer ID signed, hardened, notarized, and stapled `nanika-<version>-macos-<arch>.zip`.
- Every immutable versioned archive has a sibling SHA-256 file. Include only the Tauri application, CLI, validated built-in extension executables, required resources, and third-party notices. No extension-supplied frontend code or development tooling ships.

Create `tooling/release` around the existing Tauri bundle and target-triple sidecar build. Derive the built-in inventory from reviewed host-owned manifests, pair every executable with its manifest, and verify final signed artifact contents. Reject missing, duplicate, mismatched, or extension-asserted built-in identity. Windows uses the system Evergreen WebView2 runtime; test missing-runtime recovery on a clean profile. macOS uses the Tauri application bundle and a minimum system version of 13.0, not a hand-assembled app or Mac App Store artifact.

## Release gates

1. Verify a clean tree, pinned toolchains and lockfiles, `just check`, architecture and contract tests, and production asset inventory.
2. Measure the actual desktop application against [performance](performance.md) targets on both reference platforms.
3. Validate [UI](ui.md) behavior with computer-use and physical Windows and macOS acceptance, including IME, accessibility, Settings, extension views, failure isolation, and high DPI.
4. Verify packaged Isolation, command permissions, Content Security Policy, resource protocols, absence of remote or extension-supplied executable frontend content, and third-party notices.
5. Verify zero-extension startup, built-in/external equivalence, concrete failure reporting, and no dropped accepted work or undocumented automatic recovery.
6. Sign and verify executables and archives; notarize and staple macOS bundles; verify checksums and final built-in inventory.
7. Extract on clean user profiles and test first run, summon, configuration, actions, diagnostics, removal, startup, and rollback before publishing archives, checksums, and release notes together.

Updates replace the complete stopped application, preserving external user data. Retain the prior signed artifact and checksum for rollback. Define forward-only migrations, backup requirements, and minimum rollback versions only when a released schema first changes.
