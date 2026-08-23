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
4. Complete the platform acceptance list in `performance.md` on physical Windows and macOS machines.
5. Build with release credentials and verify signatures, notarization, archive contents, and SHA-256 files.
6. Install each artifact on a clean user profile and verify startup, summon, settings, actions, and removal.
7. Publish the versioned archives, checksums, and release notes together.

## Update and rollback

Updates replace the application files only while Nanika is stopped. User configuration and generated data stay outside the application directory. Retain at least the previous signed artifact and its checksum. Rollback stops Nanika and restores that complete artifact; never mix host and built-in extension binaries from different versions. Database migrations are forward-only, so a release that changes persistent schemas must document its minimum rollback version and backup requirements.
