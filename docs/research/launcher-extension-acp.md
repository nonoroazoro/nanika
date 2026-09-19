# Nanika extension registration, ACP sessions, and launcher chat

Date: 2026-09-18.

Current-code follow-up: [Implementation review and optimization priorities](current-implementation-review.md).

Status: Deferred research. Production ACP chat work has not started and is not part of the current implementation scope. Existing ACP transport and fixture code is preliminary infrastructure, not a completed product feature. This document does not authorize implementation or change the baseline in ../plan/tech-stack.md. Revalidate protocol details when ACP work begins.

## Decision

Use Nanika's package and contribution contract to install, identify, discover, configure, enable, and supervise extensions. Select one runtime protocol for each executable endpoint. An ACP endpoint speaks ACP directly to a Rust ACP client. It does not first speak Nanika messages, and ACP messages are not nested inside Nanika invoke frames.

The shared frontend renders launcher results, declarative extension views, and host-owned chat surfaces. Provider implementations and agent reasoning/tool orchestration remain extension responsibilities. Rust owns connection/session bookkeeping, authorized host services, local transcript storage, and UI projections.

For the existing Nanika product, retain Rust + Tauri 2 + Svelte 5 + TypeScript + Vite and native frontend state. Do not add Effect/Atom without a concrete frontend orchestration requirement. Small production-build experiments made Solid a credible fresh-project candidate, but did not establish sufficient product benefit to justify migrating the existing Svelte implementation. They measured emitted JavaScript rather than application memory, startup latency, or packaged product size. ACP integration and the proposed chat ownership boundaries do not require a frontend framework change.

## Verified starting point

- `engine/extension-management/src/ExtensionManifest.rs` already separates static contributions and permissions from the selected runtime protocol.
- `ExtensionProtocol.rs` already supports Nanika v1 and ACP v1.
- `engine/runtime/src/ExtensionRuntime.rs` selects the appropriate process adapter.
- Nanika uses length-prefixed JSON. ACP uses newline-delimited JSON-RPC on a separate stream.
- `AcpExtensionProcess.rs` currently initializes a connection, creates one session, and publishes text from agent-message chunks. This is not yet the full chat model.
- Current contributions contain commands, views, root search, and configuration. Package validation forbids Nanika commands/views for ACP packages.
- The documented ACP entry is an explicit `@<extension-id> <prompt>` candidate. Dismissal currently requests cancellation.
- Current initialization places the extension configuration in `session/new._meta["nanika.configuration"]`.

Installation/registration here is primarily a package and host registry operation. It is distinct from the Nanika process wire protocol. Keeping the first contract does not require an ACP executable to implement the second.

## Proposed registration contract

Add a first-class agent contribution. A partial illustrative manifest is:

```json
{
    "runtime": { "protocol": "acp", "protocolVersion": 1 },
    "contributes": {
        "agents": [
            { "id": "assistant", "title": "Example Agent" }
        ]
    }
}
```

`agents` is a proposal, not an accepted field in the current manifest. Initially constrain an ACP extension to one agent contribution bound to its existing endpoint. Do not introduce a general multi-endpoint system until an actual extension needs it.

The contribution lets the host publish an agent picker/launcher entry without starting the process. Selecting it opens the shared chat surface and activates its endpoint. Search text must not become a paid prompt merely because it was typed or a candidate was selected; submission is an explicit action.

Package identity remains host-validated. ACP `agentInfo` is peer metadata and cannot replace the installed extension identity or grant built-in privileges. Runtime capabilities describe what the peer supports, not what the user has authorized.

## Runtime sequence and ownership

1. Resolve a validated, enabled agent contribution.
2. Activate its supervised executable when the user enters the agent workflow.
3. Initialize ACP and validate protocol compatibility and optional capabilities.
4. Complete authentication if required by the peer's supported flow.
5. Create a session with an explicit working directory, or explicitly restore a supported session.
6. Submit a prompt and continuously handle updates and peer requests while it runs.
7. Apply ordered changes to Rust session state and deliver bounded projections to the authorized UI.
8. Respond to permission decisions and cancellation through ACP and retain the concrete final outcome.

The transport reader must remain able to process agent-to-client requests while a prompt is pending. Waiting for the prompt response before handling permission or host-service requests can deadlock the turn.

Use distinct host conversation and turn IDs. Keep the peer session ID opaque and associated with the agent identity and connection/session record. An active process connection is not the identity of a durable conversation. Search generation and WebView session IDs are separate concerns and cannot own chat cancellation.

Model at least conversation, turn, message/content blocks, tool calls, permission requests, session configuration, and terminal outcome. Preserve absent-field semantics for partial tool updates. Do not flatten all ACP activity into one growing string or imply that every agent session has every optional capability.

Source: [initialization](https://agentclientprotocol.com/protocol/v1/initialization), [prompt turn](https://agentclientprotocol.com/protocol/v1/prompt-turn), [tool calls](https://agentclientprotocol.com/protocol/v1/tool-calls).

## Two kinds of history

Nanika's persisted transcript supports display, local history, and diagnostics under the chosen storage policy. The agent's session context supports continuation. These are related but not interchangeable.

ACP `session/load` requires advertised support and replays history. The current specification also defines capability-gated resume and close operations. A cached local transcript does not prove that an agent can resume its context after exit. A replay must replace or reconcile the relevant transcript projection, not blindly append duplicate messages; message identifiers may be absent.

Do not recreate context by silently submitting the entire local history as a new prompt. That may repeat work, change behavior, and incur cost. When restoration is unavailable, distinguish viewing saved history from continuing the original session.

One process may serve multiple sessions where supported by the adapter and peer, but reuse must respect configuration, workspace, and authorization boundaries. Initially allow one active turn per session. Multiple simultaneous sessions and admission policy require a separate explicit product decision.

Source: [session setup](https://agentclientprotocol.com/protocol/v1/session-setup).

## Configuration and permission boundaries

Separate package/launch configuration from session configuration. Static Nanika settings describe installation or startup behavior. Agent-returned `configOptions` describe session choices such as model, mode, or reasoning level. Use standard `session/set_config_option` and authoritative returned state when supported.

ACP metadata permits namespaced extension data, but does not require arbitrary agents to understand `nanika.configuration`. A supported custom contract must be explicit. Do not report settings as applied merely because the peer accepted a session creation request. The current statement that ACP lacks a Nanika live-configuration acknowledgement does not imply that ACP has no standard session configuration API.

Keep three facts distinct: package declarations, negotiated protocol capabilities, and effective authorization. Advertise filesystem/terminal client capabilities only when their mechanisms and policy are implemented. An ACP tool update reports agent activity; it is not an instruction for Nanika to execute the same tool again. Host services run only in response to the appropriate validated client request.

Permission requests remain pending until an explicit decision, cancellation, or concrete connection failure. Validate the returned option against the actual pending request. Hiding the window neither approves nor rejects it. Process-tree containment controls lifetime; the current Nanika child-process model is not an enforceable OS sandbox.

Source: [session configuration](https://agentclientprotocol.com/protocol/v1/session-config-options), [extensibility](https://agentclientprotocol.com/protocol/v1/extensibility), [tool calls](https://agentclientprotocol.com/protocol/v1/tool-calls).

## Proposed user-visible lifecycle

| Action                          | Proposed meaning                                                                                    |
| ------------------------------- | --------------------------------------------------------------------------------------------------- |
| Hide launcher or return to root | Hide the surface; preserve the accepted turn and conversation.                                      |
| Stop                            | Request ACP cancellation; settle the turn from its actual terminal response or failure.             |
| Open another conversation       | Change the displayed conversation; do not implicitly replay or cancel the previous turn.            |
| New conversation                | Create a distinct conversation; handle any active-turn admission limit explicitly.                  |
| Release an active session       | Use a supported close operation under an explicit policy; do not equate this with deleting history. |
| Disable extension or quit       | Apply the documented process shutdown policy and preserve concrete outcomes.                        |

This is a proposed change from Nanika's current dismiss-cancels-ACP rule. Record it in the product contract before implementation. Cancellation can race with final updates; a local cancellation signal alone is not proof that the peer has stopped. Pending permission requests must receive the ACP cancellation outcome when their turn is cancelled.

Source: [prompt cancellation](https://agentclientprotocol.com/protocol/v1/prompt-turn).

## Proposed chat design constraints

These are design proposals for the deferred ACP work, not implemented behavior or measured performance guarantees. Validate them on Windows 10+ and macOS 13+ before adopting them.

- Use the existing launcher window and shared navigation shell for chat.
- Separate surface visibility from accepted work, subject to the lifecycle contract above.
- Batch rendering and IPC without losing ordered content. Measure the delivery cadence independently of input and scrolling responsiveness.
- Load conversation summaries separately from selected transcripts. Page large histories to keep the frontend working set bounded.
- Stop following streamed growth when the reader scrolls up, and provide Jump to Latest.
- Render controlled typed content with host-owned components. Agent output cannot supply executable markup.
- Keep ACP sessions persistent and bidirectional; a generation-only text stream does not cover session requests and permissions.
- Bound rendering cost for long conversations and verify it through measurement.
- Preserve remote cancellation outcomes and race semantics rather than treating a local signal as completion.
- Require explicit product decisions for context trimming, retention, provider fallback, and new-chat policies.
- Keep provider-specific behavior in extensions. The host supplies the generic ACP client and shared chat surface.

## Performance and artifact implications

Prefer static registration plus direct ACP over a permanently resident Nanika-to-ACP wrapper added only for registration. Do not add Node/Bun to the host solely for orchestration. External agents may still require their own runtime, which must be included in full-system resource accounting.

Keep a shared renderer and load optional chat presentation on demand. Stream identified deltas rather than entire transcripts. Stabilize completed messages and limit Markdown work to affected content while preserving parsing correctness. Page persisted history and bound rendered content without deleting messages. Heavy images, tool outputs, and syntax highlighting require explicit working-set design.

Hidden UI should not poll, paint, or hold a render loop. Rust must still process accepted protocol work and permission requests. A UI delivery queue must not be the only sink for a running hidden conversation. Define state/persistence and backpressure so a hidden window cannot stall the protocol merely by ceasing to acknowledge visual updates.

Do not invent idle timeouts, session eviction, automatic process restarts, retention, or transcript truncation to reach a memory target. Define such policies explicitly if measurements establish a need.

Measure launcher-only, first chat activation, streaming, long history, permission waits, hide/show, cancellation, and process exit on both supported platforms. Report host plus WebView and full process-tree memory separately. Measure actual release artifacts including bundled extensions and runtimes. No cross-platform runtime benchmark was performed for this supplement.

## Implementation order

1. Define the agent contribution and launch/session identity contract.
2. Define conversation, turn, cancellation, visibility, and history semantics.
3. Extend the Rust ACP adapter to the supported typed event/request model.
4. Define bounded session projections and authorized frontend commands.
5. Implement the shared chat surface using the current frontend baseline.
6. Validate protocol races and actual Windows/macOS resource use.

If one future package genuinely needs both native Nanika features and ACP chat, design explicitly bound separate endpoints and transports. Do not switch framing protocols on a live stdio stream. If future agents need to call Nanika extension capabilities as tools, evaluate a permission-scoped MCP bridge as a separate requirement; ACP does not automatically expose those capabilities.
