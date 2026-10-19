<script lang="ts">
import { onMount, untrack } from "svelte";
import { MediaQuery } from "svelte/reactivity";
import LoadMoreBoundary from "../ui/LoadMoreBoundary.svelte";
import { uiActivity } from "../../ui/activity";
import { TextDocumentModel } from "../../text/TextDocumentModel";
import { TextSurface } from "../../syntax/TextSurface";
import { ViewportTokens } from "../../syntax/ViewportTokens";
import { restoreViewportLayout, viewportContext } from "../../syntax/viewport-layout";
import { syntaxService } from "../../syntax/service";
import { LANGUAGE_LABELS } from "../../syntax/language-labels";
import { SyntaxStream } from "../../syntax/SyntaxStream";
import "../../syntax/tokens.css";
import type { DetailContent } from "../../generated/DetailContent";
import type { TextRead } from "../../generated/TextRead";
import type { TextDocument } from "../../generated/TextDocument";
import type { TextChunk } from "../../generated/TextChunk";
import type { SyntaxIdentity } from "../../syntax/SyntaxIdentity";
import type { SyntaxProgress } from "../../syntax/SyntaxProgress";
import type { ViewportLayout } from "../../syntax/ViewportLayout";

const { content, scope, viewport, busy, revision, onRead, onLanguageChange, title = null }: {
    content: Extract<DetailContent, { kind: "text"; }>;
    scope: string;
    viewport: HTMLElement | null;
    busy: boolean;
    revision: number;
    onRead: (request: TextRead) => Promise<TextChunk>;
    title?: string | null;
    onLanguageChange?: (document: TextDocument, language: string | null) => void;
} = $props();
let element = $state<HTMLDivElement>();
let model = $state.raw<TextDocumentModel | null>(null);
let rendered = $state(0);
let failure = $state(false);
let revoked = $state<{ key: string; revision: number; } | null>(null);
let surface: TextSurface | null = null;
let decoration: ViewportTokens | null = null;
let ready = $state.raw<SyntaxProgress | null>(null);
let restoredLayout: ViewportLayout | null = null;
let stream: SyntaxStream | null = null;
const dark = new MediaQuery("(prefers-color-scheme: dark)");
const identity = $derived<SyntaxIdentity>({
    scope,
    id: content.document.id,
    revision: content.document.revision,
    theme: dark.current ? "dark" : "light"
});
// Appearance changes invalidate token colors, not source reads or scroll state.
const key = $derived(JSON.stringify([scope, content.document.id, content.document.revision]));
// Revocation and republication may arrive in one Svelte batch. The admission key
// must change even when source identity and final availability remain unchanged.
const sourceKey = $derived(JSON.stringify([key, revoked]));
const sourceAvailable = $derived(revoked?.key !== key || revision > revoked.revision);
const enabled = $derived(content.syntax_highlighting);
const languageLabel = $derived(
    enabled && ready?.result.language
        ? LANGUAGE_LABELS[ready.result.language]
        : null
);

onMount(() =>
{
    const unsubscribe = syntaxService.onInvalidation((invalidScope, id, keepRevision) =>
    {
        if (
            invalidScope === scope && (id === undefined || id === content.document.id)
            && (keepRevision === undefined || keepRevision !== content.document.revision)
        )
        {
            // Revocation applies to source reads too, including resident cache hits.
            // Keep only the already displayed prefix until the refreshed view arrives.
            revoked = { key, revision };
            model?.dispose();
            model = null;
            failure = true;
            ready = null;
            stream?.dispose();
            decoration?.dispose();
            decoration = null;
            surface?.paint(null, 0, 0);
        }
    });
    return () =>
    {
        unsubscribe();
    };
});

$effect(() =>
{
    if (enabled && $uiActivity.visible)
    {
        return syntaxService.retain();
    }
});

$effect(() =>
{
    const target = element;
    const documentKey = sourceKey;
    if (!target || !documentKey || !sourceAvailable)
    {
        return;
    }
    return untrack(() =>
    {
        syntaxService.invalidate(scope, content.document.id, content.document.revision);
        const resident = enabled ? syntaxService.lookup(identity) : null;
        ready = resident;
        surface = new TextSurface(target);
        surface.clear();
        const context = ready?.result.ranges.length && viewport ? viewportContext(target, viewport) : null;
        model = new TextDocumentModel(content.document, content.value);
        surface.append(content.value);
        restoredLayout = ready && context ? restoreViewportLayout(surface, ready.result.ranges, context) : null;
        rendered = 1;
        failure = false;
        return () =>
        {
            model?.dispose();
            decoration?.dispose();
            decoration = null;
        };
    });
});

$effect(() =>
{
    const current = model;
    const root = viewport;
    const active = $uiActivity.visible;
    const highlight = enabled;
    const settled = !busy;
    const authority = identity;
    if (!current || !root)
    {
        return;
    }

    untrack(() =>
    {
        decoration?.dispose();
        decoration = null;
        if (!active)
        {
            current.cancelReads();
            return;
        }
        if (!highlight)
        {
            ready = null;
            surface?.paint(null, 0, 0);
            return;
        }
        if (settled)
        {
            stream = new SyntaxStream(syntaxService, authority, current, snapshot =>
            {
                // Preserve a warm prefix until the new cursor has caught up.
                if (snapshot.processed >= (ready?.processed ?? 0))
                {
                    ready = snapshot;
                    decoration?.setTokens(snapshot.result.ranges);
                }
            });
        }
        decoration = new ViewportTokens(surface!, root, restoredLayout, end => stream?.advance(end));
        restoredLayout = null;
        const resident = syntaxService.lookup(authority);
        if (resident)
        {
            ready = resident;
            decoration.setTokens(resident.result.ranges);
        }
        else
        {
            ready = null;
            surface?.paint(null, 0, 0);
        }
        decoration.paint();
    });
    return () =>
    {
        stream?.dispose();
        stream = null;
        decoration?.dispose();
        decoration = null;
    };
});

$effect(() =>
{
    onLanguageChange?.(content.document, languageLabel);
});

async function _read(index: number): Promise<number | null>
{
    const current = model;
    if (!current)
    {
        return null;
    }
    try
    {
        while (current.chunks.length <= index && !current.complete && current === model && $uiActivity.visible)
        {
            await current.readNext(onRead);
        }
        if (current !== model || !$uiActivity.visible || current.chunks[index] === undefined)
        {
            return null;
        }
        if (index === rendered)
        {
            surface!.append(current.chunks[index]!);
            rendered++;
            decoration?.paint();
        }
        failure = false;
        return revision;
    }
    catch
    {
        return null;
    }
}

function _preventEdit(event: Event): void
{
    // Native editing supplies caret movement and scoped Select All without allowing mutation.
    event.preventDefault();
}

function _copy(event: ClipboardEvent): void
{
    const selection = getSelection();
    if (!element || !event.clipboardData || !selection?.rangeCount)
    {
        return;
    }
    const range = selection.getRangeAt(0);
    if (!element.contains(range.startContainer) || !element.contains(range.endContainer))
    {
        return;
    }
    // Rendered-text serialization can normalize CRLF or add block separators.
    // Copy source nodes verbatim, without placing token markup on the clipboard.
    event.clipboardData.setData("text/plain", range.toString());
    event.preventDefault();
}
</script>

<div class="text-preview">
    {#if title}<h2>{title}</h2>{/if}
    <div
        bind:this={element}
        class="text-document"
        data-syntax-theme={enabled ? identity.theme : undefined}
        role="textbox"
        aria-label="Detail content"
        aria-readonly="true"
        aria-multiline="true"
        tabindex="0"
        contenteditable="true"
        spellcheck="false"
        onbeforeinput={_preventEdit}
        onpaste={_preventEdit}
        oncut={_preventEdit}
        ondrop={_preventEdit}
        oncopy={_copy}
    >
    </div>
    <LoadMoreBoundary
        {viewport}
        {busy}
        {revision}
        scope={sourceKey}
        cursor={model && rendered < content.document.totalChunks ? String(rendered) : null}
        onLoad={index => _read(Number(index))}
    />
    {#if failure}<p role="status">Text preview could not be fully loaded.</p>{/if}
</div>

<style>
.text-preview { position: relative; min-width: 0; }
h2 { margin: 0 0 var(--space-3); font-size: var(--font-meta); font-weight: 600; overflow-wrap: anywhere; }
/* Monospace needs a slightly smaller optical size than UI labels. Keep it fixed
   for plain, cold and cached text; decoration changes foreground color only. */
.text-document { min-width: 0; white-space: pre-wrap; overflow-wrap: anywhere; color: var(--text-primary); font: inherit; font-family: ui-monospace, "SFMono-Regular", Consolas, monospace; font-size: 0.875rem; line-height: 1.45; -webkit-user-select: text; user-select: text; }
p { color: var(--text-secondary); font-size: var(--font-meta); }
</style>
