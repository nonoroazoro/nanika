# Nanika Project Instructions

## Development

- Before the first release, treat the current design as the only baseline. Rewrite unpublished schemas and formats instead of preserving compatibility or migrations.
- Do not complete a stage with stale design, dead compatibility paths, or known technical debt.

## Performance

- Performance is a first-class requirement: keep the UI responsive and measure latency, frame pacing, and resource use.
- Keep blocking work off the UI thread and avoid continuous work or repainting while idle.

## Cross-platform

- Shared host, UI, diagnostics, protocol, and extension behavior must remain platform-neutral.
- Isolate unavoidable OS-specific behavior behind platform adapters and maintain implementations for every supported OS. Never introduce a single-OS solution into shared code.
- Validate cross-platform changes on Windows and macOS. Linux-specific behavior must not become an implicit fallback or enter shared paths unless Linux is added to the supported baseline.

## UI

- The UI must be elegant and coherent, with an explicit visual language and clear hierarchy.
- Validate the experience on Windows and macOS, including high-DPI displays.

## Animation

- Motion must be fluid, purposeful, and state-driven rather than decorative.
- Define timing, easing, interruption, and reduced-motion behavior; maintain smooth frame pacing at 60 Hz and 120 Hz where available.
