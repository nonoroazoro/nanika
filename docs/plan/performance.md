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

Latest full Windows development-machine run from 2026-08-23:

| Benchmark | Criterion estimate interval |
| --- | --- |
| Query delivery, 1,000 candidates | 424.36 to 431.89 us |
| Runtime foundation startup | 21.98 to 27.27 ms |
| Fixture extension process activation | 4.74 to 4.83 ms |
| Ranking, 1,000 candidates | 104.65 to 106.21 us |
| Ranking, 20,000 candidates | 2.51 to 2.57 ms |
| Warm application index, 500 entries | 76.51 to 78.02 ms |
| Application preselection, 10,000 entries | 2.13 to 2.21 ms |
| Clipboard load, 500 entries | 667.30 to 683.37 us |
| Render preparation, 100 rows | 8.36 to 8.48 ns |

Immediate reruns produced inconsistent relative changes, so this run is diagnostic evidence rather than a regression baseline. Criterion intervals are not request percentiles. Platform acceptance must separately record P50, P95, P99, frame-time variance, dropped frames, idle CPU, working set, database size, and thread count.

## Platform acceptance

Use a release build and collect at least 200 warm summons and 1,000 query updates. Record raw samples and the exact machine profile. Validate:

- focus, IME, transparency, pointer-monitor placement, mixed DPI, full-screen applications, and elevated foreground windows;
- global hotkey replacement and conflicts;
- foreground and background second launches;
- startup enable, disable, repair, approval, and hidden idle launch;
- application, command, script, batch, clipboard, and macOS bundle actions;
- 60 Hz and 120 Hz animation interruption and reduced motion;
- hidden idle CPU, memory, database size, and owner thread count.

Use Windows Performance Recorder or equivalent ETW tooling on Windows and Instruments on macOS for platform timing and frame pacing. Keep those captures out of the repository.
