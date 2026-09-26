# Design System

UI/UX contracts for the launcher, Settings and host-rendered extension surfaces.
Code and semantic tokens define implementation; [TODO](tasks.md) tracks unfinished work.
Prioritize responsive interaction, stable geometry and readable states.

## Direction and references

Use **Fluent 2** for design guidance and **Svelte 5 + Bits UI + plain CSS semantic
tokens** for implementation. Built-in and external extensions share one renderer.
Adapt for native conventions or concrete product needs and record the reason. Do not
adopt Fluent components/themes, Tailwind, shadcn-svelte or higher OS/WebView requirements.

1. Read the adopted contract here.
2. Consult the relevant official guidance below.
3. Inspect Fluent `packages/tokens` or `packages/web-components` for unresolved details;
   use non-React component references.
4. Research remaining discrepancies and record only the adopted decision/source.

Check existing workspace checkouts before cloning; verify remote, working tree and
upstream, then update safely with `git pull --ff-only`. Reference repositories are not
build dependencies. Cite commits for implementation-specific values; do not mirror sites.

| Topic                 | Reference                                                                                                                                                         |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Principles/layout     | [Principles](https://fluent2.microsoft.design/design-principles), [layout](https://fluent2.microsoft.design/layout)                                               |
| Color/tokens          | [Color](https://fluent2.microsoft.design/color), [tokens](https://fluent2.microsoft.design/design-tokens)                                                         |
| Typography/icons      | [Typography](https://fluent2.microsoft.design/typography), [iconography](https://fluent2.microsoft.design/iconography)                                            |
| Shape/depth           | [Shapes](https://fluent2.microsoft.design/shapes), [elevation](https://fluent2.microsoft.design/elevation), [material](https://fluent2.microsoft.design/material) |
| Motion/waiting        | [Motion](https://fluent2.microsoft.design/motion), [Wait UX](https://fluent2.microsoft.design/wait-ux)                                                            |
| Accessibility/content | [Accessibility](https://fluent2.microsoft.design/accessibility), [content](https://fluent2.microsoft.design/content-design)                                       |
| Component source      | [Web Components](https://github.com/microsoft/fluentui/tree/d27922755b/packages/web-components/src)                                                               |

## Shared controls

- Button, Switch, Select and ContextMenu wrap Bits UI; native Input/Textarea preserve
  editing, IME, selection, validation and typed refs. Feature surfaces compose them.
- Semantic tokens own light/dark colors, system fonts, spacing, geometry and motion,
  including portaled popups. Keep the CSS baseline legible without native effects.
- Preserve roles, keyboard operation, caret and fill feedback. No focus rings or
  focus-only borders; retain structural, validation and shortcut-recording borders.
- Action buttons/enabled popup options use pointer cursors. Disabled controls use
  default cursors; text/collection rows retain native semantics. Native disabled and
  aria-disabled both suppress actionable hover.
- Labels are not selectable. Inputs, textareas and copyable previews/paths retain
  native text selection. Decorative elements do not intercept pointer input.
- Busy Switches block repeat activation but retain hover, cursor and opacity with
  aria-busy. Hover affects track/thumb without an outer fill. Stable state is static.
- Select keeps focus on its combobox trigger and exposes the controlled listbox and
  aria-activedescendant; Bits owns navigation and Escape. See the
  [select-only pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-select-only/).
- Root rows and Settings use the same image component. Item icons inherit the extension
  manifest icon when omitted; explicit empty icons reserve the fixed icon slot.
  Images have no `src` while empty and stay hidden with `visibility: hidden` until
  loaded, preserving row geometry; invalid images show a neutral placeholder.

## Settings

Selection controls apply on activation. Text/number fields commit on leaving the
complete field or native form submission; focus movement inside compound editors does
not commit intermediate values. Invalid drafts remain editable. Only the accepted
field locks, and distinct edits serialize within the extension. Navigation/hiding
preserves accepted work. There is no Save/Discard footer or success notification.
See [configuration semantics](platform-architecture.md#settings-operations).

Enablement occupies a separate first group before domain configuration, using shared
rows and controls. Lifecycle status combines an 8px dot, a 6px gap and exact state text:
green available, gray disabled/transitioning, red failed. Color is never the only cue.
Settled status does not pulse. Transition labels replace settled text after one second;
completion/failure appear immediately. A newly opened page shows current state directly.

Pending field feedback appears after one second without shifting geometry. Unknown
progress uses an indeterminate bar; actual completed/total units use a determinate bar.
Only terminal completion unlocks input. Hidden UI stops animation; reduced motion uses
a static indicator. Failures appear in a dismissible top notification; concrete causes
remain in diagnostics, with saved/effective state reconciled separately.

`styles/settings.css` owns surface tokens, including portaled controls. Typography and
spacing follow Fluent ramps; compact control text, heading weight, geometry and colors
are Nanika adaptations. These are current values, not a mandate to copy Fluent components:

| Element                      | Treatment                                                                              |
| ---------------------------- | -------------------------------------------------------------------------------------- |
| Labels / descriptions        | 14px/20px and 12px/16px, regular                                                       |
| Page headings / control text | 20px/26px at 500; 13px regular                                                         |
| Rows / groups                | 8px vertical, 16px horizontal padding; 16px group gap; compound rows grow naturally    |
| Navigation / artwork         | 36px rows, 20px icons; 48px page artwork centered beside title/status                  |
| Controls                     | 32px height, 6px radius, uniform subtle border; state changes preserve geometry        |
| Select / number / shortcut   | 128px / 128px / 160px widths                                                           |
| Switch                       | 32x20px track, 40x32px hit area, 14px thumb; travel derived from inset/border geometry |
| Popup items                  | 32px rows, 6px/8px padding, 4px outer padding                                          |
| Shortcut keycaps             | Shared launcher treatment: 12px at 500, 4px radius/gaps, subtle fill                   |

Status token provenance: Fluent commit `d27922755b`
[light](https://github.com/microsoft/fluentui/blob/d27922755b/packages/tokens/src/alias/lightColorPalette.ts),
[dark](https://github.com/microsoft/fluentui/blob/d27922755b/packages/tokens/src/alias/darkColorPalette.ts),
[global colors](https://github.com/microsoft/fluentui/blob/d27922755b/packages/tokens/src/global/colors.ts).
Available maps to `colorPaletteGreenForeground3` (#107c10/#9fd89f), failed to
`colorPaletteRedForeground3` (#d13438/#e37d80), inactive to `colorNeutralForeground3`
(#616161/#adadad), light/dark respectively. Dot/text composition and state mapping are
Nanika choices; the transparent dot border preserves forced-color visibility.

## Root Search

Fixed-height virtual rows use `--row-height`; top/bottom spacers preserve full scroll
range. Only the delivered viewport/overscan creates rows and images. Arrow navigation
uses absolute positions and revision-bound range requests. ARIA exposes absolute row
position/full count; native input/IME remains intact. No stagger or animated reordering.
Disable browser scroll anchoring because the list owns spacer geometry.

One `RootSearchState` owns the displayed window, selection and scroll position. Pending
queries/local validation failures retain all three. The first completed new query
resets scroll/selection together, including empty results. Do not scroll to the top
while retaining the preceding page's spacer. Same-ranking page changes preserve
offscreen identity, but unloaded rows cannot execute. New rankings retain selection
when present in the delivered window, otherwise choose its nearest available row.
Empty results clear selection; keyboard targets become actionable when their page arrives.
Reconciliation scans the delivered window, not the full catalog.

Candidate `subtitle` is optional `{ "kind": "label" | "description", "text": string }`.
The host preserves its declared kind instead of inferring it from extension identity:

- Short labels and the right-side extension/type label remain complete on one line.
- Titles take remaining space and ellipsize only when required by those labels.
- Descriptions, including script paths, yield space to titles and may ellipsize.
- Native text hints expose full titles/descriptions.

## Shared scroll areas

All collection, Settings, directory, menu and Select scrolling uses `ScrollArea.svelte`.
Bits owns geometry, pointer capture and drag mapping; content uses native WebView
scrolling without a second momentum/wheel engine. Select composes its Viewport with
the shared viewport via `viewportProps`, retaining both ref attachments so keyboard
highlighting scrolls the real element. Read-only detail text recomputes height on
content/width changes and keeps native selection.

The visual reference is VS Code **Source Control and Extensions lists**, not editor
scrollbars, at commit `529ee19061e6723e0a640fe432e57c69d50a4f4f`:
[styles](https://github.com/microsoft/vscode/blob/529ee19061e6723e0a640fe432e57c69d50a4f4f/src/vs/base/browser/ui/scrollbar/media/scrollbars.css),
[visibility](https://github.com/microsoft/vscode/blob/529ee19061e6723e0a640fe432e57c69d50a4f4f/src/vs/base/browser/ui/scrollbar/scrollableElement.ts),
[colors](https://github.com/microsoft/vscode/blob/529ee19061e6723e0a640fe432e57c69d50a4f4f/src/vs/platform/theme/common/colors/miscColors.ts),
[Modern UI](https://github.com/microsoft/vscode/blob/529ee19061e6723e0a640fe432e57c69d50a4f4f/src/vs/workbench/contrib/modernUI/browser/modernUI.contribution.ts).
Windows/macOS share presentation; input conventions remain native.

- 10px track with a rounded 8px thumb. The 1px horizontal insets are Nanika's adaptation
  of standard 10px lists and Modern UI's 8px rounded lists.
- Hover reveals in 100ms; pointer exit immediately starts an 800ms linear fade.
  Re-entry reverses from current opacity. Scrolling outside hover reveals for 500ms;
  dragging holds visibility through pointer exit until release.
- Rest/hover/pressed colors follow VS Code light/dark defaults.
- Bits' 18px minimum thumb remains part of its drag geometry; do not override it in CSS.
  VS Code uses 20px. Large result counts cannot shrink the thumb below its minimum.
- Hidden documents cancel reveal timers/transitions. Live reduced motion removes
  transitions. Idle surfaces do not poll.

## Motion

Use CSS/Svelte motion and shared activity/preferences, without per-control preference
listeners or another scheduler. Adding an animation dependency needs demonstrated value.
Current timings are Nanika adaptations; source CSS is authoritative.

| Interaction       | Timing                            | Behavior                                             |
| ----------------- | --------------------------------- | ---------------------------------------------------- |
| General feedback  | 100ms, cubic-bezier(0.2, 0, 0, 1) | Short color feedback                                 |
| Switch            | 180ms, ease-out                   | Reverse from current position/color                  |
| Popup entry       | 140ms, cubic-bezier(0.2, 0, 0, 1) | Opacity and up to 2px translation                    |
| Popup exit        | 90ms, same curve                  | Finite, noninteractive outro                         |
| Windows caption   | 150ms, ease-out                   | Independent background opacity and glyph color       |
| Results / windows | Immediate / native lifecycle      | No list reordering or app-authored window transition |

Preserve the N hover bounce; do not add bounce to frequent controls. Animate popup
content without replacing its positioning wrapper transform. CSS and Svelte must not
own the same animated property. Execution never waits for motion; interrupted input
settles to the latest state. Follow live `prefers-reduced-motion`: nonessential motion
settles immediately and loading retains static text/status. Hidden windows do not
poll or animate. DOM activity controls visuals only; Tauri owns launcher hiding.

## Menus, focus and windows

Use one controlled menu presenter for bounded actions, groups, disabled/destructive
states, confirmation and existing shortcut hints. Callers supply target/position.
Search rows, list rows and standalone details retain an explicit context-menu action
entry, including when default execution is forbidden. Detail targets have no item ID;
split previews belong to their list route.

Bits owns navigation, typeahead, placement and dismissal. Escape closes the menu before
the owning surface handles another Escape. Outside click dismisses without restoring
focus; inactive documents remove content immediately. Restore focus only on active
surfaces. Launcher menus use `trapFocus=false` and `preventScroll=false`. No extra Actions
button, hidden trigger, Shift+F10/Menu binding or second focus manager. Keep launcher Tab behavior.

In-WebView menus create no native window. Tauri `Focused(false)` owns hide-on-blur;
DOM blur must not hide, suppress hiding or refocus the app. Windows Settings uses custom
controls/transparent rounded surfaces; macOS uses native titlebar controls/gestures.
Maximized content removes rounding. See [native presentation](platform-architecture.md#native-presentation-and-reveal).

## Windows caption controls

Three 46px grid slots form a 138px group. Visual and pointer regions coincide without
gaps, hit insets or overlap. The 44px header includes a separate 1px separator grid row;
content starts below it. Decorative SVGs ignore pointer hits. Hover changes no geometry.
Caption width follows VS Code; header/separator geometry is Nanika's adaptation.

Animate opacity on a fixed-color, non-interactive `::before` layer isolated behind each
glyph. Foreground color transitions separately; both use 150ms ease-out and interrupt
from current values. Neutral hover is black/white at 10%; close hover is
`rgb(232 17 35 / 90%)` with white foreground. Hidden/reduced-motion states settle
immediately; focus changes do not alter durations.

### Rendering constraint and regression check

Windows user testing exposed a stale painted background frame at completion of a
`background-color` transition, despite disjoint geometry and stable hover/computed-color
traces. Fixed-color layer opacity resolved the reported flicker. This does not prove
an internal WebView2/Chromium cause or a confirmed Tauri bug. Preserve this animation
structure; do not substitute hit insets, gaps, pointer debouncing, forced repaints,
permanent `will-change` or GPU-disable flags.

Move slowly across adjacent buttons and the bottom separator, reversing mid-transition.
Check rendered pixels as well as hover state, hidden settling and live reduced motion.
Use accelerated-content capture if needed; GDI can miss these painted backgrounds.
Windows evidence does not establish other platforms' native behavior.

## Action and input contracts

Rust authorizes shared Action metadata. Visual state and invocation intent never grant
extension permissions. Auxiliary actions do not count as app launches.

| Declaration                                    | Default activation | Explicit activation        |
| ---------------------------------------------- | ------------------ | -------------------------- |
| disabled                                       | Rejected           | Rejected                   |
| allow_default_execution=true, no confirmation  | Allowed            | Allowed                    |
| allow_default_execution=false, no confirmation | Rejected           | Allowed                    |
| confirmation_title set                         | Rejected           | Requires second activation |

Default permission and confirmation cannot combine. Restricted candidates remain
searchable and explicitly actionable when allowed. Menus/confirmation expire with
the reviewed target; queued input must not retarget or predict revisions. Selection
is nonblocking; blocking feedback waits for RPC completion and correlated state.
See [execution authority](platform-architecture.md#ipc-and-execution-authority).

## Validation and artwork

Validate actual Tauri input/IME, focus, menus, rapid state changes, light/dark,
accessibility, DPI, reduced motion and hidden activity on both platforms. Browser
fixtures alone do not prove native correctness. Compare latency, frame pacing,
startup, memory and hidden idle under equivalent workloads; build size alone is not
runtime evidence. Outstanding acceptance belongs in [TODO](tasks.md).

| Location                          | Ownership                                     |
| --------------------------------- | --------------------------------------------- |
| [assets/icons](assets/icons/)     | Original editable artwork                     |
| apps/extensions/built-in/*/assets | Packaged extension images                     |
| apps/desktop/shell/icons          | App icon sources and packaging variants       |
| target                            | Temporary previews, captures and measurements |

Keep originals separate from runtime exports; move exports with their consumers.
