# Release Process

Nanika uses immutable versioned artifacts. The MVP has no installer, background updater, or release channel service.

## Artifacts

| Platform | Artifact |
| --- | --- |
| Windows x86-64 | `nanika-<version>-windows-x86_64.zip` |
| macOS Apple silicon | `nanika-<version>-macos-aarch64.zip` |
| macOS Intel | `nanika-<version>-macos-x86_64.zip` |

Every archive has a sibling `.sha256` file. The Windows ZIP contains `Nanika.exe`, `nanika-cli.exe`, and the five built-in extension executables. The macOS ZIP contains `Nanika.app` with the same process boundary and `nanika-cli` under `Contents/MacOS`.

## Build and trust

Windows local package:

```powershell
./scripts/package-windows.ps1
```

An official Windows build passes `-CertificateThumbprint`; the script signs and verifies every executable with SHA-256 before archiving.

macOS local package:

```sh
./scripts/package-macos.sh
```

An official macOS build sets `NANIKA_SIGN_IDENTITY` and `NANIKA_NOTARY_PROFILE`. The script signs child executables first, signs the app with hardened runtime and a secure timestamp, verifies it, submits the ZIP through `notarytool`, staples the app, validates the ticket, and recreates the archive.

## Checklist

1. Confirm a clean tree, the intended version, and committed `Cargo.lock`.
2. Run `scripts/check.ps1` on Windows and `scripts/check.sh` on macOS.
3. Run deterministic benchmarks and compare them on the same reference machine.
4. Run the native UI benchmark on fixed Windows and macOS reference machines and retain the schema-versioned reports.
5. Complete the platform acceptance list in `performance.md` on physical Windows and macOS machines.
6. Build with release credentials and verify signatures, notarization, archive contents, and SHA-256 files.
7. Install each artifact on a clean user profile and verify startup, summon, settings, actions, diagnostics, and removal.
8. Confirm the host starts all five built-in extension processes and that one failed extension does not prevent the other features from loading.
9. Confirm user-facing failures name affected features, provide a recovery action, and do not expose internal process, protocol, path, or storage details.
10. Confirm Root Search and extension view inputs render Latin and CJK text through native IME composition, with aligned text and caret geometry, on Windows and macOS.
11. Confirm Root Search Enter and pointer activation, Up and Down selection, selected-row scrolling, query selection after reopen, and stable result publication while typing.
12. Confirm localized application names and original-name aliases are searchable. Confirm cached icons are normalized and remain responsive while scrolling on Windows and macOS.
13. Confirm calculator results appear for explicit symbolic and word operators and do not appear for plain search terms.
14. Confirm extension List, Split, Detail, filter, pagination, nested navigation, Back, scrolling, and every action style through the host-rendered protocol path.
15. Confirm Root Search, shared view components, interaction states, high-DPI rendering, and reduced motion against approved native captures on Windows and macOS.
16. Publish the versioned archives, checksums, and release notes together.

## Update and rollback

Updates replace the application files only while Nanika is stopped. User configuration and generated data stay outside the application directory. Retain at least the previous signed artifact and its checksum. Rollback stops Nanika and restores that complete artifact; never mix host and built-in extension binaries from different versions. After the first release, database migrations are forward-only, so a release that changes persistent schemas must document its minimum rollback version and backup requirements.
