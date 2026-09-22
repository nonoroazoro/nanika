# UI: Open Design and Validation

The current Svelte and Tauri code defines implemented presentation. The remaining design work must preserve one shared interface on macOS 13+ and Windows 10+, with native differences behind shell or platform adapters.

## Design direction

Nanika should remain quiet, immediate, and content-first. Search text, result identity, and the next action dominate. Native text editing, selection, IME, scrolling, and focus remain intact. Loading, active, hover, pressed, disabled, and failure states must be distinct without shifting stable geometry.

Extend semantic CSS tokens for typography, colors, spacing, geometry, elevation, motion, and interaction states. Use the operating-system font stack and plain CSS. Shared Svelte primitives should own repeated SearchInput, ResultList, ResultRow, SectionHeader, StatusBar, KeyHint, DetailPanel, EmptyState, LoadingState, and DiagnosticState presentation. Feature components must not create private design systems.

## Unfinished behavior

- Add a scoped error boundary and explicit async error handling around extension route content.
- Provide a dedicated accessible Root Search result-count announcement without moving focus. Continue using semantic controls and the combobox/listbox relationship.
- Add typed local message catalogs using the session's operating-system locale and deterministic English fallback. Test longer translations without tying identity or geometry to translated text.
- Define coherent initial, loading, empty, degraded, actionable failure, and unavailable states for each surface. Stable content stays visible until an authoritative replacement arrives; stale entries cannot execute.
- Keep every matching application accessible. Measure 1,000- and 2,000-entry catalogs in the actual WebView before deciding whether list virtualization is needed.
- Evaluate native window effects only as optional measured enhancement. Semantic CSS must remain legible when the effect is unavailable.

## Acceptance

Use computer-use in the actual Tauri application on Windows and macOS. Validate semantic roles and names, keyboard and pointer actions, IME candidate placement, focus, visual states, icon completion, route stability, errors, light and dark themes, reduced motion, operating-system text scaling, standard and mixed DPI, and 60 Hz and 120 Hz displays. Record visible behavior; component tests or a single-platform screenshot do not establish cross-platform acceptance.
