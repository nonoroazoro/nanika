# Nanika

## Goal

Build an exceptionally high-quality native launcher app.

## Core Principle

Before Nanika v1.0:

- Fundamental design flaws must be corrected, even when this requires breaking changes.
- Implement the best solution based on current evidence and understanding.
- Layering patches or workarounds onto a flawed underlying design is strictly prohibited.
- Remove obsolete implementations when their replacements are introduced.
- Keep Nanika-owned protocol, manifest, configuration, database schema and cache versions at v1.

## Requirements

### Workflow

- Follow the user's latest requirements, using current code and tests to establish implementation status.
- Leave changes unstaged unless requested. Use just dev and just check.
- Keep Bun/Node as development tooling and releases self-contained.

### Architecture

- Support only Windows 10+ and macOS 13+ through platform adapters with a shared contract.
- Rust owns search, supervision, storage, configuration and platform services. Tauri integration belongs in the desktop shell; callers own lifecycle and transaction policy.
- Limit launcher responsibility for App, Scripts and System commands to admission and handoff, then dismiss it. The OS owns System execution.
- Extensions provide domain capabilities under identical built-in and external runtime contracts. Host inventory establishes built-in identity.
- The host renders bounded declarative extension UI. Protect frontend and host-service boundaries through typed bridges, explicit permissions, Rust validation and session/instance-scoped authority.
- Preserve user data and accepted work, report failures explicitly, and bound queued work. New automatic lifecycle or data-management policies require explicit user authorization.

### UX and motion

Visual craft, interaction feel and motion are mandatory acceptance criteria alongside
functional correctness for every UI change.

- Use deliberate proportion, spacing, typography, surface hierarchy and state contrast to make Nanika lightweight, precise and recognizable. Reuse shared patterns that serve its interaction goals.
- Preserve native editing, selection, keyboard and window semantics, with immediate feedback, responsive input and stable text/layout.
- Provide purposeful, comfortable animation as required interaction feedback. Keep motion coherent and proportionate, with continuity through interruption and live reduced-motion support.
- Keep hidden UI idle and blocking work off UI/event-loop threads.

Nanika's product principles and native conventions govern how these skills are applied:

- `$find-animation-opportunities`: identify and prioritize useful motion opportunities; propose changes without modifying code.
- `$animate`: implement purposeful motion using the current stack and shared patterns.
- `$review-animations`: review existing or changed motion for interaction feel, timing, interruption, accessibility and performance.

### Validation

- Run checks appropriate to the change and preserve zero-extension, extension-equivalence, failure, security, protocol/storage and concurrency coverage.
- Exercise entry, exit, repeated input, interruption, scrolling and resizing on both supported platforms. Report untested conditions; native acceptance requires native execution evidence.
- Measure performance changes before and after under comparable workloads, including 60/120 Hz and hidden idle where relevant.
- Announce desktop control in advance, allow time for the user to stop input, and announce release afterward.

### Backlog

- Record worthwhile ideas deferred from current work in [docs/BACKLOG.md](docs/BACKLOG.md), the sole Markdown file under docs.
- Briefly explain each idea, why it is deferred and when to reconsider it.
- Read the backlog or implement its items only when explicitly requested by the user.
