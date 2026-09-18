# UI Design

## Platform implementation policy

The UI contract and design system are shared between macOS 13+ and Windows 10+. Native differences may be implemented in the desktop shell or a typed platform adapter when they improve the target platform experience, while shared Svelte components and semantic behavior remain unchanged. Linux and other platforms are unsupported and must not receive implicit fallback behavior.

Status: current pre-1.0 Tauri UI baseline. The frontend uses Svelte 5, TypeScript, Vite, pnpm, and plain CSS.

Debug builds retain Tauri's native Web Inspector: macOS uses WKWebView's inspector and Windows uses WebView2 DevTools. The launcher capability permits only the dedicated DevTools toggle in addition to Nanika's bounded application commands. Tauri supplies Command+Option+I on macOS and Ctrl+Shift+I on Windows. Release builds do not enable Tauri's `devtools` Cargo feature, so the inspector and toggle command remain excluded. Other platforms remain unsupported.

## Product character

Nanika should feel quiet, immediate, precise, and trustworthy. It is a focused desktop surface, not a dashboard or a web page inside a window.

The interface follows six principles:

1. Content first. Search text, result identity, and the next action dominate the hierarchy.
2. Controlled interaction first. Native text editing, scrolling, selection, and IME remain intact, while Nanika owns focus and product keyboard behavior.
3. One clear state. Hover, active selection, pressed, disabled, loading, and error states must never look interchangeable.
4. Stable geometry. Content does not jump when state changes, icons load, subtitles appear, or scrollbars become available.
5. Restrained expression. Color, elevation, borders, and motion communicate structure and state rather than decoration.
6. Contemporary foundations. Use current stable WebView, CSS, and Tauri presentation capabilities without making experimental effects or legacy compatibility code part of the design identity.

WebView user-agent `focus`, `focus-visible`, and `active` decoration is reset globally. Editable controls receive no extra focus or active decoration; the native caret and text selection are sufficient. A component may not add another focus, active, glow, scale, transform, or outline effect without an approved interaction requirement. Native caret, text selection, selected rows, and other existing product state remain the default indicators.

## Surface model

Nanika currently implements one primary surface and reserves one additional surface:

- Launcher: an undecorated, transparent, always-on-top Tauri window positioned on the active monitor. The frontend root owns the complete visible surface.
- Settings: not implemented yet. It will be a separate decorated Tauri window that follows normal desktop window behavior.

Root Search contains a search header and a scrollable results region. An extension list route combines its back button, search input, and optional filters in one header above the bounded content region and reserved action footer. It has no separate title row; the route and list retain accessible names. Detail-only routes keep a back button and title in the same header geometry.

The current launcher is a fixed 760 by 520 logical pixels. Its search header and outer geometry do not move when result counts or routes change; surplus content scrolls inside the bounded content region. Root Search and extension surfaces use the shared StatusBar. An extension surface omits the footer row when it has no actions.

Root Search and extension headers share the same 68 px height, 20 px horizontal inset, 16 px leading icon column, and 12 px text gap. Their search inputs use the same font size and text origin. The back button keeps a 32 px hit target centered over the icon column without widening it. Extension filters sit to the right of the search input in the same row.

Scrollable regions retain native WebView scrolling and scrollbar interaction. Windows-only desktop shell wiring requests Tauri's `FluentOverlay` style for all configured windows before creating their WebViews. macOS retains its existing configuration, WKWebView scrollbars, and system visibility preferences. Shared CSS remains unchanged, including Root Search's thin scrollbar width. This presentation enhancement requires WebView2 Runtime 125.0.2535.41 or later; older runtimes ignore the option and retain their default scrollbar. All WebViews sharing a data directory must use the same setting. No custom scrollbar library or JavaScript visibility timer is used. Other platforms remain unsupported at the existing adapter boundary. Windows runtime validation is pending; this change has not been visually verified on Windows.

## Design tokens

Define tokens as CSS custom properties and expose semantic names rather than raw visual values.

Token groups:

- Surface: launcher, raised, selected, hovered, pressed, input, detail, overlay, and scrim.
- Text: primary, secondary, muted, selected, disabled, destructive, warning, and success.
- Border: structural, subtle, and destructive.
- Typography: search, row title, row subtitle, section label, metadata, action, key hint, body, and code.
- Geometry: window radius, control radius, row radius, input height, row height, icon box, content inset, section gap, and status-bar height.
- Elevation: launcher shadow, raised row, and menu.
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
- Root Search and extension list panes scroll vertically only. Titles, subtitles, and extension section headings truncate with an ellipsis. Extension list groups constrain their grid column with `minmax(0, 1fr)` so long titles cannot widen rows or cause horizontal scrolling during keyboard selection.

## Icons

Every result icon is rendered inside a fixed square icon box. Native application images come from the operating system and are normalized by the Rust icon pipeline before presentation. The frontend preserves aspect ratio and never stretches content.

Root Search never waits for native icon acquisition. It presents candidate metadata with cached icons or fallbacks, then sends the final Host-ranked first ten entry IDs to their owning extensions as a requestless preparation hint. The Application Extension prioritizes those missing icons before continuing in batches of ten and publishes each completed batch. Result images use browser lazy loading and asynchronous decoding so off-screen rows do not compete with the visible set.

Rules:

- Request the 128 px cached variant for launcher rows and display it at 26 CSS px. On macOS, the adapter obtains the system image through `NSWorkspace.iconForFile` and draws it into a 256 px sRGB working bitmap before caching. The frontend does not recreate macOS icon styling.
- Use a fixed CSS presentation size independent of source dimensions, without padding, a background plate, or a corner mask. System-supplied corners, plates, highlights, and shadows remain in the image itself.
- Crop only fully transparent outer margins before caching, using the bounding box of every pixel with nonzero alpha. Preserve opaque black and white areas, partial alpha, and shadows; color alone never identifies expendable background. A fully transparent image is not a usable icon. Host-owned Clipboard History artwork follows the same alpha-only cropping policy; native and bundled icons never undergo automatic color-based background removal.
- Scale cropped artwork proportionally to fill the cache canvas along its longest edge, with no occupancy reduction or added margin. Center non-square artwork; only the shorter axis has the space needed to preserve its aspect ratio. The shared frontend displays the square canvas at 26 CSS px without a mask or background.
- Reserve icon space before loading to prevent text movement.
- Use a deterministic fallback with the same box geometry.
- Native application candidates use Rust-normalized raster icons. Static command and View contributions use host-owned artwork selected by typed contribution identity. Clipboard History, text and image content icons, and the file-icon fallback use the transparent artwork in `docs/design/icons/clipboard-history`. Runtime assets are 128 px PNGs derived with alpha-only cropping and Lanczos resizing; fully transparent margins introduced by downsampling are trimmed before the final fit. All four render in the same 26 CSS px icon slot without an additional tile, padding, or mask. Both cached placeholders and frontend load-failure icons are transparent document outlines without a background plate or corner mask. The frontend fallback appears only when an application has no icon URL or its image fails to load; it never sits behind a successfully loaded transparent image. Declarative extension list items identify product-owned semantic icons through the protocol; text and image map to the matching host-owned raster artwork. File entries prefer their system icon through an opaque, validated cache reference and use the file artwork when unavailable. A single-file detail shows a larger system icon and its name. A multi-file detail uses a fixed-height collection preview built from at most the first three icons and one item-count row, so preview height is independent of collection size. File details show at most the first five complete paths above Content type, one naturally wrapping path per line with additional spacing and native text selection. Additional paths are represented by a passive remaining-file count, without expansion or pagination. The item-count row reports the exact total beside an unbadged file icon. Copying always uses the complete collection; these limits affect presentation only. The frontend never infers an icon from a title or subtitle.
- Preserve each operating system's supplied icon shape. Nanika does not add per-application rounding, shadows, or masks; older macOS versions may supply different artwork from macOS 26.

## Search input

The search input is a semantic single-line text control and retains DOM focus while the launcher is open.

Behavior:

- Opening an empty launcher places the caret at the start.
- Returning from an extension or reopening after invoking a result selects the complete non-empty query once.
- Hiding and reopening Root Search without invoking a result preserves the current caret and selection so input can continue. Further editing clears a pending selection mark.
- Text input, selection, caret movement, clipboard shortcuts, undo, redo, dead keys, and IME remain native WebView behavior.
- The application root captures product keys before WebView defaults. Tab never performs browser focus traversal anywhere in the launcher.
- When the launcher regains focus on either macOS or Windows, the shared frontend sends one platform-neutral `resumed` lifecycle event to the active extension view. The owning extension decides whether its data must be synchronized; shared UI and shell code never special-case clipboard behavior.
- Up and Down control the result list without moving the text caret.
- Ctrl+Up and Ctrl+Down navigate input history.
- Enter invokes the active result.
- Unmodified Tab invokes the active Root Search result only when its manifest or dynamic Candidate classifies it as a View. Tab uses the same primary action as Enter and click; extensions never receive a Tab-specific action. Every other Tab or Shift+Tab combination has no action.
- Escape closes the current route or launcher according to navigation depth.
- Composition events do not trigger incomplete query execution. Search updates follow committed input state.

The input has no decorative inner panel, focus border, glow, or scale effect. Focus is communicated by the native caret and text selection.

## Result list

Root Search uses the ARIA combobox pattern with a listbox and options. DOM focus stays in the search input and `aria-activedescendant` identifies the active option. Nanika does not provide browser focus traversal, Vim-style bindings, or a general full-keyboard navigation mode.

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
- Root Search has one active option shared by keyboard and pointer navigation. Pointer movement transfers that active option.
- Pointer click activates the clicked result directly.
- Keyboard and pointer state use the same action identity.
- Unmodified F5 refreshes dynamic search contributors only while Root Search is visible and focused. Preserve the query and current list while refreshing, show a status, and apply results through the existing Channel. Ignore key repeat and additional F5 presses while an operation is pending. F5 never reloads the WebView or refreshes an extension view; it is not a global shortcut. macOS keyboards may require Fn+F5 to emit F5.

Use the browser scroll container as the source of truth. Keyboard navigation calls `scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" })` on the active option. The browser determines whether scrolling is needed and the minimum distance; do not duplicate its geometry checks or maintain a parallel pixel scroll model. Pointer selection does not request scrolling. DOM focus remains in the input, so changing `aria-activedescendant` alone does not reveal an off-screen option, as explained in the [W3C combobox guidance](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-autocomplete-list/#accessibilityfeatures).

## Extension views

Every launcher, Settings, diagnostic, and extension view surface is rendered by the shared Svelte frontend. Native tray, menu, and pre-WebView recovery surfaces remain shell-owned operating-system integration. Extensions provide bounded declarative content through the versioned extension protocol, and the frontend maps semantic view types to product-owned components. Built-in and external extensions use exactly the same rendering path.

- List: search or filter controls, sections, options, pagination, and actions.
- Split: list selection on the left and detail for the active item on the right.
- Clicking a list item restores search input focus, including when that item is already selected. Unmodified Up and Down from a read-only text preview also restore search focus and navigate the list; modified arrows retain native text-selection behavior.
- Clipboard History `Clear history` removes records matching both the selected content type and current search text, across all pages. Other records remain. With All Types and an empty query, it clears all history. The button remains an action of the selected item and is absent when the filtered list is empty.
- Detail: title, bounded body, metadata, and actions.
- Settings: planned typed controls generated from host-owned state and each extension's static `contributes.configuration` schema. The runtime registry exists, but the Settings window and frontend controls do not.

In a Split view, the list is subordinate navigation and occupies 38% of the content width; the detail pane receives 62%. Text detail uses a readonly auto-sized text area only to preserve native selection, but removes its border, radius, fill, and resize chrome so it reads as text rather than an input or image. Single-file detail uses a semantic file row; multi-file detail uses a bounded collection summary rather than repeating the complete collection before metadata. Image detail sizes its preview within the space remaining after metadata and padding, with `object-fit: scale-down` to preserve the whole image without enlarging small sources. The preview retains a minimum height; genuinely oversized metadata remains scrollable. The three content types must remain distinguishable without relying on labels alone.

Collection previews use at most three equally sized transparent artwork slots without a card background, border, corner mask, or shadow. Artwork fits without cropping or changing its aspect ratio. Items paint in increasing stacking order from left to right; slight rotation and limited overlap preserve recognition of each item. Stacking is isolated to the preview. This presentation is shared by Windows and macOS and uses the existing cached images without extra image generation, animation, or filters.

Full-window surfaces (launcher, extension view, and fatal error) have no CSS outer shadow: the WebView bounds would clip it and leave tinted pixels outside the rounded corners. Interior cards and overlays can retain their shadows. Native window shadows remain a shell concern.

The detail pane owns vertical scrolling; its article uses natural content height without a nested scroll container. The article's last child has no bottom margin; article padding provides the bottom spacing. Content that exceeds the available height, including long paths and multiple files, remains scrollable.

Clipboard's initial view performs no file-icon I/O: it immediately renders the first ten matching rows with in-memory icon results and file fallbacks. A sentinel requests the next ten rows before the user reaches the current boundary; no manual load-more control is presented. A failed page can be requested again only after the sentinel leaves and re-enters the viewport, preventing an automatic retry loop. Filtering and search always run against the complete retained history before pagination is applied. Only the selected row contributes full detail content or an image preview URL. After each view response, a dedicated extension worker schedules at most the first three files needed by the selected collection preview and only the first file in each other visible row, selected entry first. It progressively accepts work up to its bounded queue capacity, resolves persistent cache hits or acquires missing icons, and emits a coalesced view invalidation when an icon becomes available. `Resumed` is a no-op because the platform watcher continuously owns external clipboard capture; extension-requested clipboard writes remain revision-gated and do not re-enter capture. Initial extension presentation therefore remains independent of filesystem metadata latency, native icon latency, selected collection size, payload size outside the selected detail, and the total number of retained entries.

Root Search and host-rendered extension views share one `StatusBar` presentation. Its leading region is empty unless the current surface exposes contextual secondary actions such as Clipboard History's scoped Clear history action. Its trailing region displays only shortcuts that the current surface actually handles, using an action label and compact keycaps. An entry can be a pointer action or a passive shortcut status, but it never joins browser Tab traversal. Root Search refresh is passive status text: only F5 starts refresh, and the entry changes to the animated pending state while refresh runs. The bar never displays the application or extension identity. Extension actions remain declarative data; the shared renderer owns layout, interaction state, accessibility, pending presentation, and shortcut rendering.

Extension list hover is a lower-priority preview state and does not change the selected item. A selected row or pressed filter keeps its selected surface while hovered.

Extensions cannot provide markup, styles, scripts, Svelte components, WebView preload code, URLs for executable content, DOM access, Tauri access, or arbitrary drawing. They cannot select raw colors, spacing, typography, elevation, animation, or platform-specific widgets. All pixels, visible states, interactions, accessibility behavior, and motion belong to the shared frontend design system.

Extension content remains data. Text is escaped, icon references are resolved through validated host-owned protocols, declarative nodes are bounded and schema-validated, and typed actions are returned to the owning extension through the Rust runtime. No extension package is loaded as a frontend bundle.

Nested navigation uses a stable route stack. Back is visually quiet and placed consistently. It submits a typed Back request and retains the current route until the authoritative navigation snapshot arrives; failure remains visible on that route. Route updates preserve local input and selection when their stable identities remain valid.

## Status bar

The status bar is contextual and compact. It is absent when the current surface has no status or action to present.

Extension primary actions appear as passive text with an Enter key hint, matching the launcher Refresh hint. They are not clickable buttons and have no pending animation. Enter and item activation continue to invoke the primary action, subject to the existing busy guard. Secondary actions remain explicit buttons.

Launcher surfaces do not assign `tabindex`. The global keyboard handler prevents native Tab and Shift+Tab focus traversal; Root Search retains its explicit Tab action for View results. Pointer presses anywhere in the status bar preserve the current editing focus, while button clicks still invoke their actions. List views focus their search input and expose selection through `aria-activedescendant`; the outer window container is not a focus target.

- The primary action appears first in reading order and closest to its key hint.
- Secondary actions remain visually subordinate.
- Destructive actions use destructive color only when available and actionable.
- Key hints describe shortcuts and never resemble unlabeled buttons.
- A pending entry replaces its label with the concrete operation state, shows a shared progress indicator, blocks repeated activation, and announces the state without moving focus. Reduced-motion preferences stop repeated animation.
- Status entries are declarative data with one surface-level action dispatcher. Stable keyed entries preserve DOM identity; entry arrays update only when their visible data changes. Progress animation is paused whenever the document is hidden or the launcher loses focus.
- Root Search static-contribution identity icons use host-owned artwork with fixed geometry. Text and image content icons use distinct artwork from the same visual set. File content prefers the operating system icon, with the file artwork reserved for unavailable icons.
- Pressed styling ends when pointer or key activation ends. Actions do not retain an active visual state after invocation.

## States

Every screen defines these states before completion:

- Initial.
- Loading.
- Ready with content.
- Ready without content.
- Partial capability failure.
- Actionable failure.
- Unavailable capability.

Loading does not replace stable results with an empty flash. Existing content remains until the next coherent snapshot is ready. A failure identifies the unavailable capability in plain language and makes its complete technical cause available through diagnostics.

Search text is submitted through Tauri `invoke`; one Channel per page lifetime supplies all search states and result lists. RPC responses do not change the list. Initial empty-query results appear without typing, and later discovery updates replace them through the same Channel. Typing during startup is preserved. Old session, request, or revision messages cannot replace the current view.

While a new request is pending, keep the previous completed view visible, including its empty-result message, and mark the results region busy. Retained entries cannot execute until results for the current request arrive. Before any search has completed, leave the result content empty. Do not display a searching message or alternate loading and empty-result views on keystrokes. Keep list nodes keyed by result identity and isolate collection rendering from request metadata updates. An empty result does not permit the host to skip longer queries: extensions can produce new results when an expression or command becomes complete. Runtime startup failures, communication failures, and render failures show an accessible error. Nanika does not convert slow work into failure through a hidden frontend deadline.

## Theme

Support light and dark operating-system themes through the same semantic token set. Theme changes must not reload the frontend or reset navigation state.

Contrast must meet WCAG AA for text and essential controls. Selected and hovered rows remain legible in both themes. Selected and hovered surfaces stay neutral. Accent color is reserved for explicit primary actions or other approved semantic use.

## Localization

The shell currently exposes the normalized operating-system locale in session metadata but renders hard-coded English because message catalogs are not implemented. When catalogs are added, user-facing shell text follows the operating-system locale when Nanika ships a match and falls back to English otherwise. Catalog selection must not alter component geometry, focus, query state, or navigation state. Use browser `Intl` for locale-sensitive values and test longer translations instead of reserving layout for one language.

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

Do not run polling timers, repeating timers, or animation frames while the launcher is hidden or visually stable. Outstanding operations are driven by completion, explicit cancellation, or transport closure rather than frontend watchdog timers.

Use CSS transitions only for approved hover, pressed, opacity, and transform changes. Use Svelte's built-in transition, animation, and motion facilities only when component lifecycle or coordinated state requires them. The initial design system has no third-party animation library.

## Accessibility

- Use semantic controls before ARIA.
- Every interactive element has an accessible name. The active surface keeps DOM focus in its text input or explicit surface root; pointer actions do not create a separate keyboard traversal model.
- Root Search exposes input value, expanded state, its listbox, and the active option. An explicit accessible result-count announcement is not implemented yet.
- Current diagnostics use `alert` or `status` semantics. Result-count changes do not yet have a dedicated live-region announcement.
- Reading order follows visual order. Browser Tab order is intentionally disabled by the launcher-level keyboard contract.
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
