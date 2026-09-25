# UI and UX: Fluent 2

The single UI/UX design reference for the launcher, Settings and host-rendered
extension surfaces. Code and manifests define implemented behavior; [tasks](tasks.md)
records unfinished candidates without committing to implementation. Update this
document in place instead of adding component plans or implementation diaries.

## Direction and references

Keep Nanika quiet, immediate and content-first. Preserve stable geometry while
communicating hover, selection, pressed, pending, disabled and failure states.
Built-in and external extensions use the same renderer and interaction contracts.

Nanika uses **Fluent 2 as its design system**. The implementation stack is
**Svelte 5 + Bits UI (headless) + plain CSS and Nanika semantic tokens**.

| Layer             | Responsibility                                                                          |
| ----------------- | --------------------------------------------------------------------------------------- |
| Svelte 5          | Rendering, reactive state and component composition                                     |
| Bits UI           | Headless primitive behavior, keyboard interaction and accessibility semantics           |
| Fluent 2          | Design system for UX patterns, interaction states, layout, motion and design parameters |
| Nanika CSS/tokens | Implementation of that design in product styles, light/dark colors and shared values    |

Follow [Fluent 2](https://fluent2.microsoft.design/design-principles) for most UI/UX
work, including interaction patterns, states, layout and motion. Adapt only for
native conventions or concrete product needs; document the reason. Adopt applicable
official values with source attribution and identify local adaptations explicitly.

Implement the design with shared Bits UI wrappers and semantic CSS tokens, without
Fluent components/themes, Tailwind or shadcn-svelte. Where Bits UI lacks a needed
primitive, use a small native wrapper with the same contract. Business components
compose these controls. Application-level styling and accessibility still require
validation. Preserve Windows 10+ and macOS 13+ WebView support.

Use CSS/Svelte motion with shared reduced-motion behavior. Adding an animation
library requires a separate decision.

## Reference lookup workflow

Use this order for UI/UX research:

1. Read this document for Nanika's adopted rules, values and product adaptations.
2. Open the relevant official Fluent 2 page directly using the topic index below.
3. Inspect the local Fluent UI source when exact token values, component state styles
   or implementation details remain unclear.
4. Use web search for unresolved questions, official discussions and known issues.

The existing reference checkout on this workstation is `D:\Workspace\fluentui`,
from [microsoft/fluentui](https://github.com/microsoft/fluentui). Treat this as a
local research location, not a build dependency or a required path on other machines.
Follow the global repository lookup rules: verify the remote, working tree and
upstream before updating with `git pull --ff-only`; preserve local changes and
report when the checkout cannot be updated safely. If absent, check other candidate
checkouts before cloning into a dedicated system temporary directory.

Start source inspection with `packages/tokens` for concrete token values and
`packages/web-components` for concrete component parameters. Do not use the
React implementation as reference. Treat source styles as options to evaluate in
Nanika, rather than as a complete visual prescription. The repository does not replace the
website's design guidance. If guidance and implementation differ, record the
specific discrepancy and the reason for Nanika's choice.

| Topic | Official entry points |
| --- | --- |
| Principles and layout | [Design principles](https://fluent2.microsoft.design/design-principles), [Layout](https://fluent2.microsoft.design/layout) |
| Color and tokens | [Color](https://fluent2.microsoft.design/color), [Design tokens](https://fluent2.microsoft.design/design-tokens), [Color tokens](https://fluent2.microsoft.design/color-tokens) |
| Typography and icons | [Typography](https://fluent2.microsoft.design/typography), [Iconography](https://fluent2.microsoft.design/iconography) |
| Shape and depth | [Shapes](https://fluent2.microsoft.design/shapes), [Elevation](https://fluent2.microsoft.design/elevation), [Material](https://fluent2.microsoft.design/material) |
| Motion | [Motion](https://fluent2.microsoft.design/motion) |
| Accessibility and content | [Accessibility](https://fluent2.microsoft.design/accessibility), [Content design](https://fluent2.microsoft.design/content-design) |
| Component parameters | [Non-React Web Components](https://github.com/microsoft/fluentui/tree/d27922755b/packages/web-components/src) |

Keep adopted rules, parameters, adaptation reasons and source links in the relevant
sections of this document. When an implementation detail depends on repository
state, cite the source file and commit. Consult the live website for visual and
motion examples. Do not maintain a full-site mirror as part of the current workflow;
add focused, attributed reference notes only when repeated lookups justify them.

## Document and artwork ownership

| Location                         | Ownership                                                             |
| -------------------------------- | --------------------------------------------------------------------- |
| [assets/icons](assets/icons/)    | Original high-resolution icon artwork for editing and export          |
| apps/extensions/built-in/*/assets | Packaged extension images loaded through the shared resource protocol |
| apps/desktop/shell/icons         | Application icon source and platform packaging variants used by Tauri |
| target                           | Temporary previews, screenshots, probes and measurement output        |

Keep originals distinct from optimized exports. Documentation artwork is not a
runtime dependency; move or rename exports only together with their consumers.

## Shared controls and visual language

Root search results use their declared package image or cached file icon. Results without
an item icon use their extension's manifest icon, rendered by the same shared icon
component as Settings. This rule applies equally to built-in and external extensions.

- Shared Button, Switch, Select and ContextMenu wrap Bits UI. Shared native Input
  and Textarea preserve editing, IME, selection, validation and typed DOM refs.
- Feature surfaces consume these controls. StatusBar, result rows and forms retain
  business layout; they do not implement separate primitive behavior or styling.
- Root-level tokens own light/dark colors, typography, spacing, geometry, elevation
  and motion, including portaled popups. Use system fonts, compact layouts and
  legible states without relying on optional native effects.
- Menus and Select share surface, spacing, selection and separator treatment.
  Long labels retain accessible names and do not overlap shortcut hints.
- Action buttons and enabled popup options use a pointer cursor. Disabled controls
  use the default cursor; editable text and collection rows retain native semantics.
  Both native disabled and aria-disabled suppress actionable hover feedback.
- Preserve semantic roles, keyboard operation, caret and fill feedback. Do not add
  focus rings or focus-only borders. Keep structural, validation and recording borders.
- Labels are not text-selectable. Inputs, textareas and explicitly copyable content,
  including Clipboard previews and file paths, support native selection.
- Switch hover affects the track or thumb, with no surrounding button fill.
  Decorative parts do not intercept pointer input. A Switch marked busy blocks
  repeat activation while retaining cursor, opacity and hover treatment and
  exposing aria-busy. Settings locks only the field with an accepted operation.
- General and extension settings apply immediately, following
  [Fluent Switch guidance](https://fluent2.microsoft.design/components/web/react/core/switch/usage).
  Text and number edits commit when focus leaves the complete setting field or
  native form submission occurs; selection controls commit on activation. Internal
  focus movement in compound fields does not submit an intermediate value.
  Navigation and native hiding retain accepted work. No Save/Discard footer or
  saving/success notification is shown.
- Extension enablement occupies a separate first settings group, followed by the
  extension's domain configuration. Both use the same row and control treatment.
  Group spacing expresses the different responsibilities, following Fluent's
  [spacing and proximity guidance](https://fluent2.microsoft.design/layout).
  This grouping is a Nanika product decision, not a Fluent-specific extension pattern.
- Extension lifecycle status pairs a fixed 8px dot with visible text at a 6px gap, following
  Fluent [Badge](https://fluent2.microsoft.design/components/web/react/core/badge/usage)
  guidance. Green means available (running or ready on demand), gray means disabled
  or transitioning, and red means failed. Text preserves the exact state, so color
  is never the only cue. The 8px diameter, 6px gap and assignment of lifecycle
  states to three colors are Nanika choices. Values were verified against
  `microsoft/fluentui` commit `d27922755b`:
  [Badge sizes and ghost colors](https://github.com/microsoft/fluentui/blob/d27922755b/packages/react-components/react-badge/library/src/components/Badge/useBadgeStyles.styles.ts),
  [light palette](https://github.com/microsoft/fluentui/blob/d27922755b/packages/tokens/src/alias/lightColorPalette.ts),
  [dark palette](https://github.com/microsoft/fluentui/blob/d27922755b/packages/tokens/src/alias/darkColorPalette.ts),
  and [global colors](https://github.com/microsoft/fluentui/blob/d27922755b/packages/tokens/src/global/colors.ts).
  `--status-available` maps to `colorPaletteGreenForeground3` (#107c10 / #9fd89f),
  `--status-failed` to `colorPaletteRedForeground3` (#d13438 / #e37d80), and
  `--status-inactive` to `colorNeutralForeground3` (#616161 / #adadad), with light
  then dark values. The transparent border preserves the dot in forced colors.
  The dot plus separate caption is Nanika's composition, not the React component.
  Keep stable status static: a running extension is not a loading operation.
  Only color changes use the shared short control transition, respecting hidden
  activity and reduced motion. Do not fade status text or continuously pulse dots.
  Following [Wait UX](https://fluent2.microsoft.design/wait-ux), a transition label
  replaces the last settled label only after one second. Completion and failure
  appear immediately and cancel the pending label; runtime state is not delayed.
  A newly opened page without a previous settled state shows the current state.
  The existing field progress indicator supplies feedback for longer operations.
- Settings uses Fluent's Web [type ramp](https://fluent2.microsoft.design/typography):
  row labels are Body 1 (14px/20px), descriptions are Caption 1 (12px/16px), and
  page titles are Subtitle 1 (20px/26px). Shared controls retain their compact text
  treatment and 32px minimum height. Fluent's
  [medium Input implementation](https://github.com/microsoft/fluentui/blob/master/packages/react-components/react-input/library/src/components/Input/useInputStyles.styles.ts)
  also uses 32px; a control height is not a setting row height.
  General and extension rows share 8px vertical and 16px horizontal padding;
  groups are separated by 16px. These values come from Fluent's spacing ramp.
  Their assignment to Settings is a Nanika density decision, not a Fluent mandate:
  an ordinary single-line row is 48px before borders. Descriptions and compound
  editors grow naturally instead of clipping to a fixed row height.
- Pending feedback appears after one second below the affected control, with
  stable geometry. Unknown progress uses a small indeterminate bar; real
  completed/total work units use a determinate bar. Progress never unlocks input;
  only the terminal result does. Hidden UI stops animation and reduced motion
  uses a static indicator. Follow Fluent's
  [progress information hierarchy](https://fluent2.microsoft.design/components/web/react/core/progressbar/usage),
  not its animation implementation. Motion and compact positioning are Nanika adaptations.
- Settings failures appear in a compact top notification with explicit dismissal.
  There is no field-level retry button or persistent error paragraph. The original
  cause remains in diagnostics; saved and effective values are reconciled separately.

## Settings component treatment

Settings uses Fluent's spacing, proximity and alignment principles with a restrained
native desktop treatment. `styles/settings.css` owns surface-specific semantic
tokens, including portaled controls; shared controls retain their input contracts.

- Row labels remain 14/20px and descriptions 12/16px. Navigation and control text
  use 13px regular for compact desktop reading. Page headings are 20/26px at weight
  500; section headings and selected navigation also use 500. The 13px control
  size and lighter heading weights are product adaptations, not the Web type ramp.
- Body and control text use regular weight. Shortcut keycaps use the shared
  launcher treatment: subtle backgrounds, 12px text at weight 500, 4px corner
  radii and 4px gaps around separators. Settings has no keycap overrides. Primary text is soft charcoal in light
  mode and off-white in dark mode; secondary text retains readable contrast.
- Navigation icons and their containers share 20px. Page artwork uses 48px to
  balance the 46px title/status stack (26px title, 4px gap, 16px status), centered
  vertically as one heading group.
  Navigation rows are 36px. Existing 8px/16px form padding and the separate first
  enablement group preserve compact density and responsibility boundaries.
- Form controls share a 32px height, 6px radius and a uniform subtle border. Theme
  has no emphasized bottom edge. Hover and pressed states change fill/border without
  changing geometry. A 20px line plus two 1px borders leaves 5px vertical insets.
- Select and numeric controls use 128px width; the shortcut recorder uses 160px for
  key combinations. The 16px chevron uses an 8px-wide stroke drawing.
- Switches retain a compact filled 32x20px track and 40x32px hit area. The 14px thumb
  has 2px internal insets plus the 1px border on both ends; travel derives from that
  geometry. This compact treatment is a Nanika adaptation. Stable state remains
  static and the existing interruption/reduced-motion/hidden-activity rules apply.
- Popup items use 32px rows with 6px/8px padding and 4px outer padding. Colors,
  border opacity and popup shadows are deliberate surface choices, not verbatim
  Fluent token mappings. Assess them in complete light/dark pages at native DPI.
- Select exposes the select-only combobox role and the controlled listbox ID because
  Bits keeps DOM focus on its trigger and highlights options via aria-activedescendant.
  Keyboard selection and Escape remain owned by Bits; see the
  [ARIA select-only pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-select-only/).

Use [Fluent typography](https://fluent2.microsoft.design/typography) and
[layout guidance](https://fluent2.microsoft.design/layout) to guide hierarchy and
rhythm. Matching individual constants does not establish visual quality. Review
alignment, optical size, text weight, borders and interaction states together.

## Motion

Choose duration and easing for the interaction, using the official
[Fluent motion guidance](https://fluent2.microsoft.design/motion) and applicable
component tokens as the design baseline. Small controls need immediate feedback;
start-fast, end-slow motion suits the Switch. Avoid a universal easing curve.

Current CSS values below are **Nanika adaptations**, not verified Fluent token
values. The stylesheet is authoritative; audit provenance before broader adoption.

| Interaction                  | Current timing                    | Behavior                                                   |
| ---------------------------- | --------------------------------- | ---------------------------------------------------------- |
| Windows caption buttons      | Immediate                         | Adjacent highlights switch without overlapping fades      |
| General control feedback     | 100ms, cubic-bezier(0.2, 0, 0, 1) | Short color feedback                                       |
| Switch position and color    | 180ms, ease-out                   | Fast start, gentle stop; reverse from the current position |
| Popup entry                  | 140ms, cubic-bezier(0.2, 0, 0, 1) | Opacity and up to 2px translation                          |
| Popup exit                   | 90ms, cubic-bezier(0.2, 0, 0, 1)  | Finite, noninteractive outro                               |
| Search results and selection | Immediate                         | No stagger or animated reordering                          |
| Window show/hide             | No app-authored transition        | Native lifecycle; no animation acknowledgement             |

Preserve the existing N hover bounce. Do not add bounce to frequent controls.
Animate popup content without replacing its positioning wrapper transform.
CSS and Svelte must not own the same animated property. Execution never waits
for an animation; rapid input settles to the latest authoritative state.

Follow prefers-reduced-motion live in every WebView. Nonessential motion becomes
an immediate final state; loading retains static text and accessible status.
Hidden windows do not poll or animate. Shared document activity controls visual
work only; Tauri owns native window hiding. Do not add a scheduler or per-control
preference listeners.

## Menus, focus and windows

Use one shared controlled menu presenter with bounded actions, groups, disabled
and destructive states, confirmation labels and hints for existing shortcuts.
Callers supply a semantic target and pointer position. Search results, list rows
and standalone detail views must all retain an explicit action entry point,
including when default execution is forbidden. Detail targets have no item ID;
a split preview belongs to its list route.

Bits owns menu navigation, typeahead, placement and dismissal. Escape closes the
menu before the owning surface handles another Escape. Outside clicks dismiss
without restoring focus to the dismissed menu. Inactive document state removes
menu content immediately. Do not add Actions buttons, Shift+F10/Menu-key bindings,
hidden triggers or a second focus manager. Launcher Tab and F5 retain their existing product behavior.

An in-WebView menu creates no native window. Tauri Focused(false) owns launcher
hide-on-blur; DOM blur must not hide, suppress hiding or refocus the app. Allow
menu focus restoration only while the surface is active. Launcher menus use
trapFocus=false and preventScroll=false.

Windows Settings uses a transparent rounded surface and custom window controls;
macOS uses native titlebar controls and gestures. Maximized content removes
rounding. Native mechanisms and file reveal remain in the shell/platform adapters,
with typed IPC, permissions and concrete errors. See [platform architecture](platform-architecture.md).

## Action and input contracts

Rust authorizes execution. Search candidates, manifest commands, list items and
detail views share Action metadata. Visual style never grants permission.

| Declaration                                    | Default activation | Explicit activation          |
| ---------------------------------------------- | ------------------ | ---------------------------- |
| disabled                                       | Rejected           | Rejected                     |
| allow_default_execution=true, no confirmation  | Allowed            | Allowed                      |
| allow_default_execution=false, no confirmation | Rejected           | Allowed                      |
| confirmation_title set                         | Rejected           | Requires a second activation |

Default permission and confirmation cannot be combined. Restricted candidates
remain searchable and explicitly invokable when allowed. Invocation intent never
grants an extension capability. Auxiliary actions do not count as app launches.

Menus and confirmation bind to the reviewed target and expire when it changes.
Queued input must not retarget actions or guess revisions. Selection remains
nonblocking; blocking view-action feedback waits for RPC completion and correlated
state delivery. See [execution contracts](platform-architecture.md#ipc-and-execution-authority).

## Validation

Validate actual Tauri input, IME, focus, menus, rapid state changes, light/dark,
accessibility, DPI, reduced motion and hidden activity on both platforms. Browser
fixtures do not establish native correctness; remaining gaps live in [tasks](tasks.md).

Measure replacement cost after removing superseded code: production JS/CSS,
startup, interaction latency, frame pacing, memory and hidden idle under comparable
workloads. Source size and selective imports alone do not establish performance.
Temporary previews and measurements belong under target.
