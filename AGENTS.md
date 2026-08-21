# Nanika Project Instructions

## Performance

- Performance is a first-class requirement: keep the UI responsive and measure latency, frame pacing, and resource use.
- Keep blocking work off the UI thread and avoid continuous work or repainting while idle.

## UI

- The UI must be elegant and coherent, with an explicit visual language and clear hierarchy.
- Validate the experience on Windows and macOS, including high-DPI displays.

## Animation

- Motion must be fluid, purposeful, and state-driven rather than decorative.
- Define timing, easing, interruption, and reduced-motion behavior; maintain smooth frame pacing at 60 Hz and 120 Hz where available.
