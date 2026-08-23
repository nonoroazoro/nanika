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

The first Windows baseline used Windows 10 22H2 build 19045, Ryzen 7 5800X, 32 GiB RAM, and GeForce RTX 3080. A physical macOS baseline is still required.

## Deterministic benchmarks

Run `scripts/benchmark.ps1`. Criterion covers ranking, query delivery, runtime foundation startup, application indexing and preselection, extension process activation, calculator evaluation, clipboard persistence, and render preparation. Use `-Baseline <name>` only for comparisons on the same reference machine.

## Native UI benchmark

Build `nanika-host` in release mode, then run `scripts/benchmark-native-windows.ps1`. The harness verifies hidden startup, hidden-idle CPU, working set, thread count, repeated summon and dismissal, and warm activation P50, P95, P99, maximum, and raw samples. It writes a schema-versioned JSON report under `target/performance` with the commit, worktree state, executable and lockfile hashes, machine profile, parameters, thresholds, and result.

The default 200-sample run is a Windows pre-release gate. Shorter runs only validate the harness. The equivalent report must be implemented and run on a physical Mac before 1.0.

Windows development-machine measurements for commit `2d00bd3` from 2026-08-23:

| Benchmark | Criterion estimate interval |
| --- | --- |
| Query delivery, 1,000 candidates | 353.35 to 356.63 us |
| Runtime foundation startup | 19.12 to 19.65 ms |
| Fixture extension ready | 35.55 to 36.48 ms |
| Fixture extension shutdown | 1.34 to 1.44 ms |
| Fixture extension lifecycle | 36.94 to 37.22 ms |
| Ranking, 1,000 candidates | 91.08 to 92.68 us |
| Ranking, 20,000 candidates | 2.22 to 2.26 ms |
| Warm application index, 500 entries | 65.51 to 66.51 ms |
| Application preselection, 10,000 entries | 1.65 to 1.68 ms |
| Clipboard load, 500 entries | 522.87 to 533.08 us |
| Render preparation, 100 rows | 8.02 to 8.08 ns |

The extension benchmark separates ready, shutdown, and full lifecycle timing. Windows ready time includes suspended creation, Job Object containment, and handshake. The previous 4.8 ms result predates startup containment and is not comparable. Immediate reruns still vary with background load, so these results are diagnostic evidence rather than a fixed regression baseline. Criterion intervals are not request percentiles. Platform acceptance must separately record P50, P95, P99, frame-time variance, dropped frames, idle CPU, working set, database size, and thread count.

A later current-head run was invalid for comparison because a game was active, CPU load reached 70%, and the machine used the balanced power plan. Do not use its Criterion changes as a regression baseline.

The commit `2d00bd3` packaged clean-profile smoke recorded a 15-second hidden-idle sample with the host and five built-in extension processes: 1.458% of one logical core, 175.09 MiB combined working set, 77 threads, and 110,688 bytes of database files. On the 16-thread reference machine this is about 0.091% of total logical CPU capacity. Treat it as diagnostic evidence until repeated under the fixed idle conditions above.

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
