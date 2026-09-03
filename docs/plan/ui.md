# UI Design

Status: current pre-1.0 Tauri UI baseline. The frontend uses Svelte 5, TypeScript, Vite, pnpm, and plain CSS.

## Product character

Nanika should feel quiet, immediate, precise, and trustworthy. It is a focused desktop surface, not a dashboard or a web page inside a window.

The interface follows six principles:

1. Content first. Search text, result identity, and the next action dominate the hierarchy.
2. Native behavior first. Text editing, focus, scrolling, selection, IME, and accessibility follow established platform and web semantics.
3. One clear state. Hover, active selection, keyboard focus, pressed, disabled, loading, and error states must never look interchangeable.
4. Stable geometry. Content does not jump when state changes, icons load, subtitles appear, or scrollbars become available.
5. Restrained expression. Color, elevation, borders, and motion communicate structure and state rather than decoration.
6. Contemporary foundations. Use current stable WebView, CSS, and Tauri presentation capabilities without making experimental effects or legacy compatibility code part of the design identity.

## Surface model

Nanika has two primary surfaces:

- Launcher: an undecorated, transparent, always-on-top Tauri window positioned on the active monitor. The frontend root owns the complete visible surface.
- Settings: a separate decorated Tauri window that follows normal desktop window behavior.

The launcher contains three structural regions:

1. Search header.
2. Scrollable content region.
3. Contextual action bar, rendered only when it contains useful actions or status.

The launcher height follows content up to a bounded maximum. Empty space is not used as decoration. The resting scrollport ends after a complete row, and the action bar never creates a blank footer. Resizing caused by result-count changes must not move the search header.

## Design tokens

Define tokens as CSS custom properties and expose semantic names rather than raw visual values.

Token groups:

- Surface: launcher, raised, selected, hovered, pressed, input, detail, overlay, and scrim.
- Text: primary, secondary, muted, selected, disabled, destructive, warning, and success.
- Border: structural, subtle, focus, and destructive.
- Typography: search, row title, row subtitle, section label, metadata, action, key hint, body, and code.
- Geometry: window radius, control radius, row radius, input height, row height, icon box, content inset, section gap, and action-bar height.
- Elevation: launcher shadow, raised row, menu, and focus ring.
- Motion: fast, standard, exit, easing-standard, easing-emphasized, and reduced-motion overrides.

Components may combine tokens but must not introduce private color systems or unrelated spacing scales.

## Typography

Use the operating-system UI font stack. Preserve native glyph selection for Latin, CJK, emoji, and symbols. Do not ship a custom font for brand character before platform testing proves it necessary.

Typography rules:

- Search text is the largest type on the launcher but must remain visually centered inside a single-line input.
- Result titles use medium emphasis through size and color, not excessive font weight.
- Subtitles and metadata are secondary and never reduce title legibility.
- Section labels are quiet navigation aids, not page headings.
- Static text uses explicit line height. Editable text uses a semantic single-line control with tested font, padding, and box metrics; do not simulate its caret or selection.
- Truncation uses one line and ellipsis unless the view contract explicitly permits multi-line content.

## Icons

Every result icon is rendered inside a fixed square icon box. Source images are normalized by the Rust icon pipeline before presentation. The frontend preserves aspect ratio and never stretches content.

Rules:

- Request the 128 px cached variant for launcher rows.
- Use a fixed CSS presentation size independent of source dimensions.
- Center transparent and non-square artwork optically through the normalized source canvas.
- Reserve icon space before loading to prevent text movement.
- Use a deterministic fallback with the same box geometry.
- Do not apply per-application rounding, shadows, or masks unless the operating-system source already contains them.

## Search input

The search input is a semantic single-line text control and retains DOM focus while the launcher is open.

Behavior:

- Opening an empty launcher places the caret at the start.
- Reopening with a non-empty query selects the complete value.
- Text input, selection, caret movement, clipboard shortcuts, undo, redo, dead keys, and IME remain native WebView behavior.
- Up and Down control the result list without moving the text caret.
- Ctrl+Up and Ctrl+Down navigate input history.
- Enter invokes the active result.
- Escape closes the current route or launcher according to navigation depth.
- Composition events do not trigger incomplete query execution. Search updates follow committed input state.

The input has no decorative inner panel. Focus is communicated by the native caret and a restrained focus treatment on the search region.

## Result list

Root Search uses the ARIA combobox pattern with a listbox and options. DOM focus stays in the search input and `aria-activedescendant` identifies the active option.

Each row has stable slots:

- Icon.
- Primary title.
- Optional subtitle next to or below the title according to the approved density.
- Optional right-aligned category or accessory.

The icon and text block share one left alignment axis across every row. Titles and subtitles never drift toward the visual center when accessory content is absent.

Selection contract:

- The first result is active when a non-empty result snapshot arrives unless a stable active identity remains present.
- Up and Down move exactly one option and clamp at list boundaries.
- Moving inside the visible scrollport does not change scroll position.
- When the active option crosses the top or bottom scrollport edge, reveal only the minimum required amount.
- Pointer hover does not steal keyboard selection.
- Pointer click activates the clicked result directly.
- Keyboard and pointer state use the same action identity.

Use the browser scroll container as the source of truth. Do not maintain a parallel pixel scroll model. Prefer `scrollIntoView({ block: "nearest" })` only after an actual boundary check. Do not animate selection-following scroll.

## Extension views

Extensions provide bounded declarative content. The frontend maps semantic view types to shared components.

- List: search or filter controls, sections, options, pagination, and actions.
- Split: list selection on the left and detail for the active item on the right.
- Detail: title, bounded body, metadata, and actions.
- Settings: typed controls generated from the shared settings schema.

Extensions cannot provide markup, styles, scripts, URLs for executable content, or arbitrary drawing. All visible states and interactions belong to the shared frontend design system.

Nested navigation uses a stable route stack. Back is visually quiet, placed consistently, and returns immediately. Route updates preserve local input and selection when their stable identities remain valid.

## Action bar

The action bar is contextual and compact. It is absent when it has no useful content.

- The primary action appears first in reading order and closest to its key hint.
- Secondary actions remain visually subordinate.
- Destructive actions use destructive color only when available and actionable.
- Key hints describe shortcuts and never resemble unlabeled buttons.
- Pressed styling ends when pointer or key activation ends. Actions do not retain an active visual state after invocation.

## States

Every screen defines these states before completion:

- Initial.
- Loading.
- Ready with content.
- Ready without content.
- Partial capability failure.
- Recoverable action failure.
- Unavailable capability.

Loading does not replace stable results with an empty flash. Existing content remains until the next coherent snapshot is ready. Diagnostics identify the unavailable capability and provide one concrete recovery action without exposing internal protocol or process terminology.

## Theme

Support light and dark operating-system themes through the same semantic token set. Theme changes must not reload the frontend or reset navigation state.

Contrast must meet WCAG AA for text and essential controls. Selected and hovered rows remain legible in both themes. Blue is reserved for focus, links, or explicit accent use rather than filling every selected surface.

## Localization

User-facing shell text follows the operating-system locale when Nanika ships a matching bundled catalog and falls back to English otherwise. Catalog selection must not alter component geometry, focus, query state, or navigation state. Use browser `Intl` for locale-sensitive values and test longer translations instead of reserving layout for one language.

Application titles use the localized names supplied by the application extension, while original names remain searchable aliases. Never concatenate translated fragments or use text as an action, component, or persistence identity.

## Platform materials

The semantic CSS surface defines the complete visual hierarchy. Stable Tauri native window effects may add platform material, depth, or translucency through Windows- and macOS-specific configuration, but they never carry essential contrast or state.

An effect is accepted only when physical validation proves that it improves the launcher in both light and dark environments without startup flash, border artifacts, resize artifacts, focus-state discontinuity, excessive compositor cost, or unreadable content. Unsupported, disabled, or degraded effects fall back to the semantic surface without changing geometry.

## Motion

Motion is state-driven and interruptible.

- Summon communicates appearance and focus without delaying interaction.
- Dismissal is brief and can be interrupted by a new summon.
- Selection changes do not animate position.
- Scroll following keyboard selection is immediate.
- Loading indicators animate only while work is active.
- `prefers-reduced-motion` disables non-essential movement and reduces essential transitions to immediate state changes.

Do not run timers or animation frames while the launcher is hidden or visually stable.

Use CSS transitions for hover, pressed, focus, opacity, and transform changes. Use Svelte's built-in transition, animation, and motion facilities only when component lifecycle or coordinated state requires them. The initial design system has no third-party animation library.

## Accessibility

- Use semantic controls before ARIA.
- Every interactive element has an accessible name and visible focus behavior.
- Root Search exposes input value, expanded state, result count, and active option.
- Dynamic diagnostics and result-count changes use restrained live-region announcements.
- Keyboard order follows visual order.
- Pointer targets remain usable at supported operating-system scaling levels.
- Color is never the only indicator of selection, failure, or destructive intent.

## Cross-platform validation

Use one shared design and allow only evidence-backed platform adjustments. Validate on physical Windows and macOS systems for:

- system fonts and CJK fallback;
- IME composition and candidate-window placement;
- focus and active-monitor placement;
- standard, Retina, and mixed-DPI displays;
- mouse, trackpad, scrollbar, and keyboard scrolling;
- light and dark themes;
- operating-system text and accessibility scaling;
- 60 Hz and 120 Hz displays;
- WebView2 and WKWebView behavior.

No component is complete from a single-platform screenshot.
