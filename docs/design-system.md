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

## Document and artwork ownership

| Location                         | Ownership                                                             |
| -------------------------------- | --------------------------------------------------------------------- |
| [assets/icons](assets/icons/)    | Original high-resolution icon artwork for editing and export          |
| apps/desktop/frontend/src/assets | Optimized runtime images imported by shared UI components             |
| apps/desktop/shell/icons         | Application icon source and platform packaging variants used by Tauri |
| target                           | Temporary previews, screenshots, probes and measurement output        |

Keep originals distinct from optimized exports. Documentation artwork is not a
runtime dependency; move or rename exports only together with their consumers.

## Shared controls and visual language

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
