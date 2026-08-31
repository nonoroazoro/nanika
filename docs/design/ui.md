# Nanika UI Design

Status: design proposal. This document defines the visual and interaction target for the next UI implementation stage. The existing functional UI remains the implementation baseline until each proposal is implemented and validated.

## Design direction

Nanika uses a restrained graphite interface with cool blue accents, compact information density, and clear keyboard-first hierarchy. Technology character comes from precise spacing, typography, motion, and state transitions rather than glow, decorative gradients, or dense chrome.

The UI must remain coherent across Windows and macOS while respecting platform font metrics, IME behavior, scale factors, accessibility, and reduced-motion settings. Shared visual behavior belongs to the host design system. Unavoidable platform differences stay behind platform adapters.

## Root Search proposal

[Open the interactive Root Search proposal](./root-search.html).

The proposal includes two connected states:

- Root Search with a single integrated query surface, sectioned results, full-row selection, semantic icons, secondary descriptions, result accessories, and a route-local action bar.
- Clipboard History entered through its Root Search command, rendered as a host-owned split List and Detail view with local filtering, Back navigation, item metadata, and typed actions.

The mockup is interactive:

- Type to filter Root Search results.
- Use Up and Down to move selection.
- Press Enter on Clipboard History to open its extension route.
- Press Escape or use Back to return to Root Search.

## Visual foundation

The initial component system uses these targets:

| Element | Target |
| --- | --- |
| Window surface | Dark graphite, subtle cool elevation, restrained border |
| Spacing | 8 px base rhythm with optical adjustment where required |
| Root Search header | Approximately 72 to 82 px, integrated input and mode context |
| Result row | Approximately 56 to 62 px, full-row interactive state |
| Icon surface | Approximately 36 to 38 px with one semantic icon |
| Surface radius | 12 to 18 px depending on hierarchy |
| Row radius | 8 to 11 px |
| Primary text | High-contrast system UI text with moderate weight |
| Secondary text | Blue-gray, clearly subordinate without becoming illegible |
| Accent | Cool blue-violet used only for focus, selection, and status |
| Action bar | Persistent route-local primary action and keyboard hints |

Exact values become design tokens during implementation. They must be tested at native scale factors rather than copied as fixed CSS assumptions.

## Host and extension ownership

The host owns pixels and interaction consistency:

- Layout primitives and responsive constraints.
- Typography, colors, borders, elevation, icons, focus, and selection rendering.
- Keyboard navigation, IME, accessibility, action presentation, and motion.
- Shared Root Search and declarative List, Split, and Detail components.

Extensions own semantics:

- Commands, candidates, titles, descriptions, categories, and icon references.
- Declarative view content, sections, selection identity, filters, pagination, and actions.
- State transitions caused by typed extension events.

Extensions do not receive arbitrary drawing access or control host styling. Layout choice remains semantic and bounded by the extension protocol, while the host resolves it into a coherent platform-neutral presentation.

## Motion

Motion must communicate state change and remain interruptible:

- Selection feedback: 80 to 100 ms ease-out.
- Route transition: 120 to 160 ms, interrupted immediately by Back, dismissal, or a new summon.
- Dismissal: preserve the existing 110 ms state-driven contract until measured evidence justifies a change.
- Reduced motion: snap to the final state without transitional movement.
- Idle: no continuous repaint.

## Implementation stages

1. Introduce host-owned design tokens and reusable `egui` components without changing the extension architecture.
2. Rebuild Root Search using the component system and a bounded presentation model for title, subtitle, category, icon, and accessory.
3. Rebuild the declarative List, Split, and Detail renderers using the same components.
4. Add empty, loading, degraded, and diagnostic states.
5. Validate high-DPI rendering, keyboard navigation, IME, accessibility, reduced motion, latency, and frame pacing on Windows and macOS.

The proposal is not complete implementation. Missing states and platform acceptance remain TODO until explicitly implemented and tested.
