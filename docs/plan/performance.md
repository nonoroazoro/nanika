# Performance Validation

Performance results are evidence, not pass or fail gates on ordinary machines. Compare results only on the same hardware, power mode, display topology, build profile, and background load.

## Targets

- Warm summon to interactive overlay: P95 at or below 50 ms.
- Input to updated result state: P95 at or below 16.7 ms.
- Stable 60 Hz and 120 Hz frame pacing where available.
- Hidden idle: no continuous repaint and near-zero CPU.
- No filesystem, SQLite, or blocking extension work on the UI thread.

## Reference machines

| Profile | Minimum record |
| --- | --- |
| Windows reference | Windows 10 22H2, x86-64 CPU, 16 GiB RAM, dedicated or integrated GPU, mixed-DPI dual monitors |
| macOS reference | macOS 13 or later, Apple silicon, 16 GiB RAM, Retina display plus one external display |

## Deterministic benchmarks

Run `scripts/benchmark.ps1`. Criterion covers ranking, query delivery, runtime foundation startup, application indexing and preselection, extension process activation, calculator evaluation, clipboard persistence, and render preparation. Use `-Baseline <name>` only for comparisons on the same reference machine.

## Native UI benchmark

Build `nanika-host` in release mode, then run `scripts/benchmark-native-windows.ps1`. The harness verifies hidden startup, hidden-idle CPU, working set, thread count, repeated summon and dismissal, and warm activation P50, P95, P99, maximum, and raw samples. It writes a schema-versioned JSON report under `target/performance` with the commit, worktree state, executable and lockfile hashes, machine profile, parameters, thresholds, and result.

The default 200-sample run is a Windows pre-release gate. Shorter runs only validate the harness. The equivalent report must be implemented and run on a physical Mac before 1.0.

The extension benchmark separates ready, shutdown, and full lifecycle timing. Windows ready time includes suspended creation, Job Object containment, and handshake. Criterion intervals are not request percentiles. Platform acceptance separately records P50, P95, P99, frame-time variance, dropped frames, idle CPU, working set, database size, and thread count.

## Platform acceptance

Use a release build and collect at least 200 warm summons and 1,000 query updates. Keep the native JSON report and record the exact machine profile. Validate:

- focus, IME, pointer-monitor placement, mixed DPI, full-screen applications, and elevated foreground windows;
- global hotkey replacement and conflicts;
- foreground and background second launches;
- startup enable, disable, repair, approval, and hidden idle launch;
- application, command, script, batch, clipboard, and macOS bundle actions;
- 60 Hz and 120 Hz animation interruption and reduced motion;
- hidden idle CPU, memory, database size, and owner thread count.

Use Windows Performance Recorder or equivalent ETW tooling on Windows and Instruments on macOS for platform timing and frame pacing. Keep those captures out of the repository.
