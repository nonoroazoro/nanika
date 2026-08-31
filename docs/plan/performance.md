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

## Activation trace

The host activation trace covers native hotkey delivery, host event-queue handling, monitor placement, preparation, render submission, native visibility, and focus acquisition. The passive platform observer reads Carbon `EventTime` on macOS and `MSG.time` on Windows before `global-hotkey` discards the native timestamp, then always continues native event propagation. Slow activations above 50 ms are warning-level diagnostics. Set `NANIKA_DIAGNOSTICS=verbose` to retain all activation samples.

The trace reports both `hotkey_delivery_ms` and `callback_to_focus_ms`. `total_ms` includes both segments when the native timestamp is available. `timing_complete=false` identifies a hotkey sample that must not be used as end-to-end evidence.

## Native UI benchmark

Build `nanika-host` in release mode, then run `scripts/benchmark-native-windows.ps1`. The harness verifies hidden startup, hidden-idle CPU, working set, thread count, repeated summon and dismissal, and warm activation P50, P95, P99, maximum, and raw samples. It writes a schema-versioned JSON report under `target/performance` with the commit, worktree state, executable and lockfile hashes, machine profile, parameters, thresholds, and result.

The default 200-sample run is a Windows pre-release gate. Shorter runs only validate the harness. A project-owned macOS black-box harness must be implemented and run on a physical Mac before 1.0. Synthetic `CGEventPost` input is valid only after the harness proves that Carbon receives the events and the host trace records the matching activations on the tested macOS version.

### macOS overlay lifecycle regression record

On 2026-08-26, a release build was tested on macOS 26.4.1 with an Apple M5 Pro and 48 GiB of memory. The black-box harness posted `Ctrl+Space`, observed the native window, dismissed it with Escape, waited 1 ms, and immediately posted `Ctrl+Space` again. It used a locally modified copy of Asyar `benchtool.swift` at revision `af3658b2467cf972d79c6388a24e0bb2b6f73fe3`. This validation code is GPL-3.0 and is not part of the Nanika source tree.

| Samples | Median | P95 | Minimum | Maximum | Timeouts |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 20 | 31.1 ms | 39.3 ms | 23.4 ms | 39.9 ms | 0 |

The matching internal activation traces included one warmup and reported 21 callback-to-focus samples with a 10.4 ms median, 9.4 ms mean, and 15.9 ms maximum. These internal samples had `timing_complete=false`, so the black-box result is the end-to-end evidence. The previous failure reproduced as approximately 1,000 ms between physical input and Carbon callback after a real native hide.

A second 10-run sequence used `Ctrl+Space` for both show and hide. Every toggle succeeded. Hide-to-not-visible had a 140.5 ms median and 149.9 ms maximum, including the intentional 110 ms dismissal animation, synthetic input, AX polling, and compositor visibility confirmation.

The parked and hidden release process used 0.16% CPU over a 30-second sample while verbose diagnostics were enabled. This is a regression sample, not the final 60-second release acceptance measurement.

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
