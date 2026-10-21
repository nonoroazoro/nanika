<script lang="ts">
import { flushSync, onMount } from "svelte";
import { SvelteMap } from "svelte/reactivity";
import Preview from "../../src/components/extensions/TextPreview.svelte";
import ScrollArea from "../../src/components/ui/ScrollArea.svelte";
import { fixtures } from "./fixtures";
import { syntaxScenarios } from "./scenarios";
import { instrumentWorkers } from "./instrument-workers";
import { verifyViewportColors } from "./verify-viewport";
import { verifySelection } from "./verify-selection";
import { measureLatency, recordLatencyRead } from "./latency";
import { syntaxService } from "../../src/syntax/service";
import { SYNTAX_FINGERPRINT } from "../../src/syntax/fingerprint";
import { LANGUAGE_LABELS } from "../../src/syntax/language-labels";
import { TOKEN_BATCH_CHARS } from "../../src/syntax/limits";
import "../../src/styles/global.css";
import type { DetailContent } from "../../src/generated/DetailContent";
import type { TextDocument } from "../../src/generated/TextDocument";
import type { TextRead } from "../../src/generated/TextRead";
import type { TextChunk } from "../../src/generated/TextChunk";
import type { SyntaxFixture } from "./SyntaxFixture";

instrumentWorkers();

const enabled = new URLSearchParams(location.search).get("syntax") === "on";
const prepared = fixtures.map(fixture =>
{
    const characters = Array.from(fixture.text);
    const chunks = Array.from(
        { length: Math.max(1, Math.ceil(characters.length / 16_384)) },
        (_, index) => characters.slice(index * 16_384, (index + 1) * 16_384).join("")
    );
    const lines = fixture.text.split("\n");
    return {
        ...fixture,
        chunks,
        document: {
            id: fixture.id,
            revision: "1",
            utf16Length: fixture.text.length,
            lineCount: lines.length,
            maxLineLength: Math.max(...lines.map(line => line.endsWith("\r") ? line.length - 1 : line.length)),
            totalChunks: chunks.length
        }
    };
});
const navigationFixtures = Array.from({ length: 36 }, (_, index) =>
{
    const base = prepared.find(item => item.id === ["markdown-1000", "typescript-1000", "dense-unicode"][index % 3])!;
    const id = `rapid-${index}`;
    return { ...base, id, document: { ...base.document, id } };
});
const byId = new SvelteMap([...prepared, ...navigationFixtures].map(item => [item.id, item]));
let content = $state<Extract<DetailContent, { kind: "text"; }>>({
    kind: "text",
    value: "",
    syntax_highlighting: enabled,
    document: { id: "empty", revision: "1", utf16Length: 0, lineCount: 1, maxLineLength: 0, totalChunks: 1 }
});
let viewport = $state<HTMLDivElement | null>(null);
let viewRevision = $state(1);
let reportedLanguage: string | null = null;
let readCount = 0;
let readBytes = 0;
let warmMisses = 0;
let firstFrameChecks = 0;
const longTasks: number[] = [];

function _stats(values: number[])
{
    const sorted = [...values].sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length * .5)] ?? 0;
    return {
        samples: sorted.length,
        p50: median,
        p95: sorted[Math.floor(sorted.length * .95)] ?? 0,
        p99: sorted[Math.floor(sorted.length * .99)] ?? 0,
        max: sorted.at(-1) ?? 0,
        aboveOneAndHalfMedian: sorted.filter(value => value > median * 1.5).length
    };
}

function _frame(): Promise<number>
{
    return new Promise(resolve =>
    {
        requestAnimationFrame(resolve);
    });
}

function _pause(milliseconds: number): Promise<void>
{
    return new Promise(resolve =>
    {
        setTimeout(resolve, milliseconds);
    });
}

function _identity(item: SyntaxFixture)
{
    return {
        scope: "benchmark",
        id: item.id,
        revision: "1",
        theme: window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" as const : "light" as const
    };
}

function _show(item: typeof prepared[number], highlight = enabled): void
{
    flushSync(() =>
    {
        viewRevision++;
        content = {
            kind: "text",
            value: item.chunks[0] ?? "",
            document: item.document,
            syntax_highlighting: highlight
        };
    });
}

function _source(): HTMLElement
{
    const source = document.querySelector<HTMLElement>(".text-document");
    if (!source)
    {
        throw new Error("Real preview component is missing.");
    }
    return source;
}

async function _read(request: TextRead): Promise<TextChunk>
{
    recordLatencyRead("source-read-start");
    const value = byId.get(request.documentId)?.chunks[request.index];
    if (value === undefined || request.revision !== "1")
    {
        throw new Error("Benchmark source is unavailable.");
    }
    readCount++;
    readBytes += value.length * 2;
    recordLatencyRead("source-read-end");
    return { ...request, value };
}

function _verifyFirstCommit(item: typeof prepared[number]): void
{
    if (!enabled)
    {
        return;
    }
    const resident = syntaxService.lookup(_identity(item));
    const expectedLabel = resident?.result.language
        ? LANGUAGE_LABELS[resident.result.language]
        : "";
    if ((reportedLanguage ?? "") !== expectedLabel)
    {
        throw new Error(`Resident language label was stale or missing: ${item.id}`);
    }
    if (!resident?.result.ranges.length)
    {
        return;
    }
    firstFrameChecks++;
    const source = _source();
    const spans = source.querySelectorAll("[class^=syntax-token-]");
    if (
        !spans.length
    )
    {
        warmMisses++;
    }
}

async function _revealAll(item: typeof prepared[number]): Promise<void>
{
    for (let attempt = 0; attempt < 64 && _source().textContent !== item.text; attempt++)
    {
        if (viewport)
        {
            viewport.scrollTop = viewport.scrollHeight;
        }
        await _pause(20);
    }
}

async function _prepareInteraction()
{
    const item = byId.get("boundary-fence")!;
    _show(item);
    await _revealAll(item);
    const source = _source();
    source.focus({ preventScroll: true });
    if (viewport)
    {
        viewport.scrollTop = 0;
    }
    await _frame();
    await _frame();
    // Exercise caret movement across a real source-block boundary.
    const start = (source.querySelector(".syntax-source-segment")?.textContent?.length ?? 300) - 3;
    const walker = document.createTreeWalker(source, NodeFilter.SHOW_TEXT);
    let remaining = start;
    for (let node = walker.nextNode(); node; node = walker.nextNode())
    {
        const length = (node as Text).length;
        if (remaining <= length)
        {
            getSelection()?.setBaseAndExtent(node, remaining, node, remaining);
            return { start, text: item.text };
        }
        remaining -= length;
    }
    throw new Error("Could not position the native caret.");
}

function _interactionState()
{
    const source = _source();
    const selection = getSelection();
    const offset = (node: Node | null | undefined, index: number): number | null =>
    {
        if (!node || !source.contains(node))
        {
            return null;
        }
        const range = document.createRange();
        range.selectNodeContents(source);
        range.setEnd(node, index);
        return range.toString().length;
    };
    return {
        anchor: offset(selection?.anchorNode, selection?.anchorOffset ?? 0),
        focus: offset(selection?.focusNode, selection?.focusOffset ?? 0),
        focused: document.activeElement === source,
        text: source.textContent
    };
}

async function _verifyViewportPresentation()
{
    if (!enabled || !viewport)
    {
        return [];
    }
    const results = [];
    for (const id of ["dense-json", "dense-unicode", "markdown-1000"])
    {
        const item = byId.get(id)!;
        const reference = await _referenceTokens(item);
        _show(item);
        await _revealAll(item);
        const source = _source();
        getSelection()?.setBaseAndExtent(source, source.childNodes.length, source, 0);
        for (const width of [560, 420])
        {
            viewport.style.width = `${width}px`;
            for (const position of [0, .5, 1])
            {
                viewport.scrollTop = (viewport.scrollHeight - viewport.clientHeight) * position;
                const scroll = viewport.scrollTop;
                const height = source.getBoundingClientRect().height;
                // Scroll dispatch and the viewport's rAF each precede worker demand.
                await _frame();
                await _frame();
                const deadline = performance.now() + 5000;
                while (_workerCounts().pending !== 0)
                {
                    if (performance.now() >= deadline)
                    {
                        throw new Error(`Viewport highlighting did not settle: ${id}`);
                    }
                    await _pause(5);
                }
                await _frame();
                const result = {
                    id,
                    width,
                    position,
                    ...verifyViewportColors(source, viewport, reference),
                    stableLayout: source.getBoundingClientRect().height === height,
                    stableScroll: viewport.scrollTop === scroll,
                    backwardSelection: _interactionState().anchor === item.text.length
                        && _interactionState().focus === 0
                };
                results.push(result);
                if (
                    result.mismatches || !result.checked || !result.stableLayout || !result.stableScroll
                    || !result.backwardSelection
                )
                {
                    throw new Error(`Viewport presentation failed: ${JSON.stringify(result)}`);
                }
            }
        }
        getSelection()?.removeAllRanges();
    }
    return results;
}

/**
 * Build a complete reference outside measurements without populating the preview cache.
 * The real preview must still request and publish its own viewport batches.
 */
async function _referenceTokens(item: typeof prepared[number]): Promise<Uint32Array>
{
    const identity = _identity(item);
    const signal = new AbortController().signal;
    const language = await syntaxService.detect(identity, item.chunks[0] ?? "", signal);
    if (!language)
    {
        throw new Error(`Reference fixture was not detected: ${item.id}`);
    }
    const stream = syntaxService.openStream();
    const ranges: number[] = [];
    try
    {
        for (let offset = 0; offset < item.text.length; offset += TOKEN_BATCH_CHARS)
        {
            const text = item.text.slice(offset, offset + TOKEN_BATCH_CHARS);
            const result = await syntaxService.append(identity, text, language, signal, {
                stream,
                offset,
                final: offset + text.length === item.text.length
            });
            if (!result)
            {
                throw new Error(`Reference tokenization failed: ${item.id}`);
            }
            ranges.push(...result.ranges);
        }
        return Uint32Array.from(ranges);
    }
    finally
    {
        syntaxService.closeStream(stream);
    }
}

function _workerCounts()
{
    const metrics = (window as unknown as {
        syntaxWorkerMetrics: { created: number; terminated: number; received: number; pending: number; };
    }).syntaxWorkerMetrics;
    return { ...metrics, live: metrics.created - metrics.terminated };
}

async function _verifyLayoutCache()
{
    if (!enabled || !viewport)
    {
        return null;
    }
    const item = byId.get("dense-unicode");
    const other = byId.get("markdown-100");
    if (!item || !other)
    {
        throw new Error("Missing layout-cache fixtures.");
    }
    viewport.scrollTop = 0;
    viewport.style.width = "560px";
    _show(item);
    await _frame();
    _show(other);
    _show(item);
    const source = _source();
    const text = source.textContent;
    const contained = source.querySelectorAll('[style*="content-visibility"]').length;
    const tokens = syntaxService.lookup(_identity(item))?.result.ranges;
    if (!tokens)
    {
        throw new Error("Layout-cache fixture has no complete token map.");
    }
    source.style.fontSize = "17px";
    await _frame();
    await _frame();
    const scaled = verifyViewportColors(source, viewport, tokens);
    const revoked = source.querySelectorAll('[style*="content-visibility"]').length === 0;
    source.style.removeProperty("font-size");
    await _frame();
    await _frame();
    const restored = verifyViewportColors(source, viewport, tokens);
    _show(other);
    _show(item);
    flushSync(() =>
    {
        content = { ...content, syntax_highlighting: false };
    });
    const disabledRevoked = _source().querySelectorAll('[style*="content-visibility"]').length === 0;
    flushSync(() =>
    {
        content = { ...content, syntax_highlighting: true };
    });
    const result = { contained, revoked, disabledRevoked, scaled, restored, sourceExact: source.textContent === text };
    if (
        (CSS.supports("content-visibility", "auto") && !contained) || !revoked || !disabledRevoked
        || !result.sourceExact
        || !scaled.checked || scaled.mismatches || !restored.checked || restored.mismatches
    )
    {
        throw new Error(`Layout cache did not follow source metrics: ${JSON.stringify(result)}`);
    }
    return result;
}

async function _verifyHiddenLifecycle()
{
    if (!enabled)
    {
        return null;
    }
    const item = byId.get("markdown-300")!;
    syntaxService.invalidate("benchmark", item.id);
    _show(item);
    for (let attempt = 0; attempt < 1_000 && !syntaxService.lookup(_identity(item)); attempt++)
    {
        await _pause(5);
    }
    const before = _workerCounts();
    const descriptor = Object.getOwnPropertyDescriptor(document, "visibilityState");
    const reads = readCount;
    let hidden;
    try
    {
        Object.defineProperty(document, "visibilityState", { configurable: true, value: "hidden" });
        flushSync(() => document.dispatchEvent(new Event("visibilitychange")));
        const started = _workerCounts();
        _show(byId.get("boundary-fence")!);
        await _pause(200);
        const stopped = _workerCounts();
        hidden = {
            simulatedVisibility: true,
            hadWorker: before.live === 1,
            liveWorkers: stopped.live,
            newWorkers: stopped.created - started.created,
            messages: stopped.received - started.received,
            reads: readCount - reads
        };
    }
    finally
    {
        if (descriptor)
        {
            Object.defineProperty(document, "visibilityState", descriptor);
        }
        else
        {
            Reflect.deleteProperty(document, "visibilityState");
        }
        flushSync(() => document.dispatchEvent(new Event("visibilitychange")));
    }
    _show(item);
    const warmRestore = !!_source().querySelector("[class^=syntax-token-]");
    if (!hidden.hadWorker || hidden.liveWorkers || hidden.newWorkers || hidden.messages || hidden.reads || !warmRestore)
    {
        throw new Error(`Hidden lifecycle failed: ${JSON.stringify({ ...hidden, warmRestore })}`);
    }
    return { ...hidden, warmRestore };
}

async function _verifySourceRevocation()
{
    if (!enabled || !viewport)
    {
        return null;
    }
    return [await _verifyDocumentRevocation(false), await _verifyDocumentRevocation(true)];
}

async function _verifyDocumentRevocation(wholeScope: boolean)
{
    const base = prepared.find(item => item.chunks.length > 1)!;
    if (viewport)
    {
        const id = `revocation-${wholeScope}`;
        const item = { ...base, id, document: { ...base.document, id } };
        byId.set(id, item);
        viewport.scrollTop = 0;
        _show(item);
        const deadline = performance.now() + 5000;
        while (!syntaxService.lookup(_identity(item)) || _workerCounts().pending)
        {
            if (performance.now() >= deadline)
            {
                throw new Error("Revocation preview did not produce its prefix cache");
            }
            await _pause(5);
        }
        if (syntaxService.lookup(_identity(item))!.processed > item.chunks[0]!.length)
        {
            throw new Error("Revocation preview cached more than its source prefix");
        }
        const prefix = _source().textContent;
        const reads = readCount;
        flushSync(() => syntaxService.invalidate("benchmark", wholeScope ? undefined : id));
        viewport.scrollTop = viewport.scrollHeight;
        await _frame();
        await _frame();
        const revoked = _source().textContent === prefix && readCount === reads
            && !document.querySelector(".load-more-boundary") && !syntaxService.lookup(_identity(item));
        // Fresh view publication can republish unchanged immutable bytes.
        _show(item, false);
        await _revealAll(item);
        const restored = _source().textContent === item.text;
        const loadedReads = readCount;
        flushSync(() =>
        {
            content = { ...content, syntax_highlighting: true };
        });
        await _frame();
        flushSync(() =>
        {
            content = { ...content, syntax_highlighting: false };
        });
        const presentationPreservedSource = _source().textContent === item.text && readCount === loadedReads;
        // Coalesced delivery must recreate admission even when availability stays true.
        flushSync(() =>
        {
            syntaxService.invalidate("benchmark", wholeScope ? undefined : id);
            viewRevision++;
        });
        await _revealAll(item);
        const batchedRepublication = _source().textContent === item.text && readCount > loadedReads;
        const result = { wholeScope, revoked, restored, presentationPreservedSource, batchedRepublication };
        if (!revoked || !restored || !presentationPreservedSource || !batchedRepublication)
        {
            throw new Error(`Source revocation failed: ${JSON.stringify(result)}`);
        }
        return result;
    }
    throw new Error("Missing revocation test viewport.");
}

async function _prepareCache()
{
    const preparation = [];
    if (enabled)
    {
        for (const item of prepared)
        {
            if (item.language === null)
            {
                continue;
            }
            const started = performance.now();
            if (viewport)
            {
                viewport.scrollTop = 0;
            }
            _show(item);
            const deadline = started + 5000;
            while (!syntaxService.lookup(_identity(item)) || _workerCounts().pending !== 0)
            {
                if (performance.now() >= deadline)
                {
                    throw new Error(
                        `Preview did not finish its first screen: ${
                            JSON.stringify({
                                id: item.id,
                                processed: syntaxService.lookup(_identity(item))?.processed,
                                workers: _workerCounts(),
                                language: reportedLanguage
                            })
                        }`
                    );
                }
                await _pause(5);
            }
            const snapshot = syntaxService.lookup(_identity(item))!;
            if (snapshot.processed > item.chunks[0]!.length)
            {
                throw new Error("Preview cached more than its first source chunk");
            }
            preparation.push({
                id: item.id,
                milliseconds: performance.now() - started,
                processed: snapshot.processed,
                ranges: snapshot.result.ranges.length / 3
            });
        }
    }
    return preparation;
}

async function _measureRendering()
{
    const preparation = await _prepareCache();
    let observer: PerformanceObserver | null = null;
    if (PerformanceObserver.supportedEntryTypes.includes("longtask"))
    {
        observer = new PerformanceObserver(list =>
        {
            longTasks.push(...list.getEntries().map(entry => entry.duration));
        });
        observer.observe({ entryTypes: ["longtask"] });
    }
    try
    {
        const groups = [];
        const matrix: Array<[string, typeof prepared]> = [
            ["4k", prepared.filter(item => item.id.endsWith("-4096"))],
            ["16k", prepared.filter(item => item.id.endsWith("-16384"))],
            ["markdown", prepared.filter(item => /^markdown-\d+$/.test(item.id))],
            ["density", prepared.filter(item => item.id.startsWith("dense-"))],
            ["svelte", prepared.filter(item => item.language === "svelte")]
        ];
        for (const [name, items] of matrix)
        {
            const frames: number[] = [];
            const commits: number[] = [];
            const fixtureCommits: Record<string, number[]> = {};
            let previous = await _frame();
            for (let index = 0; index < 120; index++)
            {
                const now = await _frame();
                if (index >= 20)
                {
                    frames.push(now - previous);
                }
                previous = now;
                const item = items[index % items.length];
                if (!item)
                {
                    throw new Error("Empty benchmark group.");
                }
                const start = performance.now();
                _show(item);
                void viewport?.offsetHeight;
                if (index >= 20)
                {
                    const elapsed = performance.now() - start;
                    commits.push(elapsed);
                    const measured = fixtureCommits[item.id] ?? [];
                    measured.push(elapsed);
                    fixtureCommits[item.id] = measured;
                }
            }
            groups.push({
                name,
                frameIntervals: _stats(frames),
                synchronousCommits: _stats(commits),
                fixtureCommits: Object.fromEntries(
                    Object.entries(fixtureCommits).map(([id, values]) => [id, _stats(values)])
                )
            });
            dispatchEvent(
                new CustomEvent("syntax-benchmark-progress", { detail: `Component group complete: ${name}` })
            );
        }
        const scrollFrames: number[] = [];
        const scrollItem = byId.get("markdown-1000")!;
        _show(scrollItem);
        await _revealAll(scrollItem);
        let previousScrollFrame = await _frame();
        for (let index = 0; index < 80; index++)
        {
            const now = await _frame();
            if (index >= 10)
            {
                scrollFrames.push(now - previousScrollFrame);
            }
            previousScrollFrame = now;
            if (viewport)
            {
                viewport.scrollTop = (index % 20) * 80;
                if (index % 20 === 0)
                {
                    viewport.style.width = index % 40 === 0 ? "420px" : "560px";
                }
            }
        }
        return {
            fingerprint: SYNTAX_FINGERPRINT,
            mode: enabled ? "on" : "off",
            preparation,
            groups,
            scrollFrameIntervals: _stats(scrollFrames),
            readBytes,
            longTasks: PerformanceObserver.supportedEntryTypes.includes("longtask") ? _stats(longTasks) : null
        };
    }
    finally
    {
        observer?.disconnect();
    }
}

async function _verifyColdInlineChart()
{
    const item = byId.get("inline-chart")!;
    syntaxService.suspend();
    syntaxService.invalidate("benchmark", item.id);
    if (viewport)
    {
        viewport.scrollTop = 0;
    }
    const workers = _workerCounts().created;
    _show(item);
    const deadline = performance.now() + 5000;
    while (
        (syntaxService.lookup(_identity(item))?.processed ?? 0) < item.text.length
        || _workerCounts().pending !== 0
    )
    {
        if (performance.now() >= deadline)
        {
            throw new Error("Cold inline HTML preview did not finish");
        }
        await _pause(5);
    }
    const snapshot = syntaxService.lookup(_identity(item))!;
    const source = _source();
    const result = {
        newWorker: _workerCounts().created === workers + 1,
        sourceExact: source.textContent === item.text,
        language: reportedLanguage,
        ranges: snapshot.result.ranges.length / 3,
        colored: !!source.querySelector("[class^=syntax-token-]"),
        // The fixture is copied source, never an executable HTML preview.
        inert: !source.querySelector("script, style, #chart-container")
    };
    if (
        !result.newWorker || !result.sourceExact || result.language !== "HTML" || !result.ranges
        || !result.colored || !result.inert
    )
    {
        throw new Error(`Cold inline HTML preview failed: ${JSON.stringify(result)}`);
    }
    return result;
}

async function _verifyPresentation()
{
    const coldInlineChart = await _verifyColdInlineChart();
    const selectionChecks = verifySelection();
    await _prepareCache();
    for (const item of prepared)
    {
        _show(item);
        _verifyFirstCommit(item);
    }
    const correctness = [];
    for (const id of ["markdown-1000", "unicode-markdown", "boundary-fence", "svelte-500"])
    {
        const item = byId.get(id);
        if (!item)
        {
            throw new Error("Missing correctness fixture.");
        }
        _show(item);
        await _revealAll(item);
        const source = _source();
        const selection = getSelection();
        selection?.selectAllChildren(source);
        const transfer = new DataTransfer();
        const copy = new ClipboardEvent("copy", { clipboardData: transfer, bubbles: true, cancelable: true });
        source.dispatchEvent(copy);
        correctness.push({
            id,
            sourceExact: source.textContent === item.text,
            selectionExact: selection?.getRangeAt(0).toString() === item.text,
            nativeSelectionNormalized: selection?.toString() !== item.text,
            copyExact: copy.defaultPrevented ? transfer.getData("text/plain") === item.text : null
        });
        selection?.removeAllRanges();
    }
    let coldPresentation = null;
    let disabledLiveWorkers: number | null = null;
    let enabledFromCache = null;
    if (enabled)
    {
        const item = byId.get("markdown-100")!;
        syntaxService.invalidate("benchmark", item.id);
        const start = performance.now();
        _show(item);
        const source = _source();
        const initialHeight = source.getBoundingClientRect().height;
        const initiallyUnlabeled = reportedLanguage === null;
        const initialTypography = _typography(source);
        const immediateText = source.textContent === item.text;
        const selection = getSelection();
        selection?.setBaseAndExtent(source, source.childNodes.length, source, 0);
        for (let attempt = 0; attempt < 1_000 && !syntaxService.lookup(_identity(item)); attempt++)
        {
            await _pause(5);
        }
        coldPresentation = {
            milliseconds: performance.now() - start,
            immediateText,
            ready: !!syntaxService.lookup(_identity(item)),
            languageLabel: initiallyUnlabeled && reportedLanguage === "Markdown",
            stableTypography: _typography(source) === initialTypography,
            tokenTypography: [...source.querySelectorAll("[class^=syntax-token-]")].every(token =>
                _typography(token) === initialTypography
            ),
            stableLayout: source.getBoundingClientRect().height === initialHeight,
            stableSelection: selection?.getRangeAt(0).toString() === item.text,
            backwardSelection: _interactionState().anchor === item.text.length && _interactionState().focus === 0
        };
        selection?.removeAllRanges();
        flushSync(() =>
        {
            content = { ...content, syntax_highlighting: false };
        });
        disabledLiveWorkers = _workerCounts().live;
        const disabledTypography = _typography(source) === initialTypography;
        if (
            _source().querySelector("[class^=syntax-token-]") || disabledLiveWorkers !== 0
            || reportedLanguage !== null
        )
        {
            throw new Error("Disabling syntax left token spans or its Worker alive.");
        }
        const retained = syntaxService.lookup(_identity(item));
        const beforeEnable = _workerCounts();
        flushSync(() =>
        {
            content = { ...content, syntax_highlighting: true };
        });
        enabledFromCache = {
            retained: !!retained,
            immediateColors: !!_source().querySelector("[class^=syntax-token-]"),
            immediateLabel: reportedLanguage === "Markdown",
            newWorkers: _workerCounts().created - beforeEnable.created,
            stableTypography: disabledTypography && _typography(source) === initialTypography
        };
        if (
            !enabledFromCache.retained || !enabledFromCache.immediateColors || enabledFromCache.newWorkers
            || !enabledFromCache.stableTypography || !enabledFromCache.immediateLabel
        )
        {
            throw new Error(`Re-enabling syntax discarded shared ready data: ${JSON.stringify(enabledFromCache)}`);
        }
    }
    const viewportPresentation = await _verifyViewportPresentation();
    const layoutCache = await _verifyLayoutCache();
    const hiddenLifecycle = await _verifyHiddenLifecycle();
    const sourceRevocation = await _verifySourceRevocation();
    if (
        warmMisses || correctness.some(item => !item.sourceExact || !item.selectionExact || item.copyExact === false)
        || (coldPresentation && (!coldPresentation.immediateText || !coldPresentation.ready
            || !coldPresentation.stableLayout || !coldPresentation.stableSelection
            || !coldPresentation.languageLabel
            || !coldPresentation.backwardSelection || !coldPresentation.stableTypography
            || !coldPresentation.tokenTypography))
    )
    {
        throw new Error(
            `Syntax component correctness gate failed: ${JSON.stringify({ warmMisses, correctness, coldPresentation })}`
        );
    }
    return {
        coldInlineChart,
        correctness,
        selectionChecks,
        firstFrameChecks,
        warmMisses,
        viewportPresentation,
        layoutCache,
        hiddenLifecycle,
        sourceRevocation,
        coldPresentation,
        disabledLiveWorkers,
        enabledFromCache
    };
}

/**
 * Exercise bounded caches and document teardown without retaining synthetic entries.
 * External heap probes collect after this function returns, outside timed work.
 */
async function _stressLifecycle()
{
    const base = byId.get("markdown-100")!;
    const empty = prepared.find(item => item.id === "plain-url")!;
    const start = _workerCounts();
    for (let index = 0; index < 160; index++)
    {
        const id = `stress-${viewRevision}-${index}`;
        const item = { ...base, id, document: { ...base.document, id } };
        _show(item);
        if (index % 4 === 0)
        {
            // Let completed work enter the LRU as well as exercising cancellation.
            for (let attempt = 0; attempt < 1_000 && !syntaxService.lookup(_identity(item)); attempt++)
            {
                await _pause(5);
            }
        }
        else
        {
            await _frame();
        }
    }
    _show(empty, false);
    await _frame();
    const end = _workerCounts();
    const result = {
        switches: 160,
        created: end.created - start.created,
        live: end.live,
        cacheEntries: syntaxService.cache.size,
        cacheBytes: syntaxService.cache.bytes
    };
    if (result.live !== 0 || result.cacheEntries > 128 || result.cacheBytes > 8 * 1024 * 1024)
    {
        throw new Error(`Lifecycle bounds exceeded: ${JSON.stringify(result)}`);
    }
    return result;
}

async function _prepareVisibility()
{
    const item = byId.get("markdown-100")!;
    syntaxService.invalidate("benchmark", item.id);
    _show(item);
    for (let attempt = 0; attempt < 1000 && !syntaxService.lookup(_identity(item)); attempt++)
    {
        await _pause(5);
    }
    if (!_source().querySelector("[class^=syntax-token-]") || _workerCounts().live !== 1)
    {
        throw new Error("Visibility scenario requires a ready preview with a live worker");
    }
    return _workerCounts();
}

function _verifyVisibility()
{
    const workers = _workerCounts();
    if (workers.live !== 0 || !_source().querySelector("[class^=syntax-token-]"))
    {
        throw new Error("Native hide/restore must stop the worker and preserve cached colors");
    }
    return workers;
}

function _typography(element: Element): string
{
    const style = getComputedStyle(element);
    return JSON.stringify([
        style.fontFamily,
        style.fontSize,
        style.fontWeight,
        style.fontStyle,
        style.lineHeight,
        style.letterSpacing
    ]);
}

function _runLatency()
{
    const empty = {
        id: "latency-empty",
        text: "",
        chunks: [""],
        language: null,
        document: { id: "latency-empty", revision: "1", utf16Length: 0, lineCount: 1, maxLineLength: 0, totalChunks: 1 }
    };
    const wrap = (item: typeof prepared[number]) => ({
        id: item.id,
        text: item.text,
        show: (highlight = true): void =>
        {
            _show(item, highlight);
        }
    });
    const rapid = navigationFixtures.map(wrap);
    return measureLatency(
        wrap(byId.get("markdown-100")!),
        wrap(byId.get("markdown-1000")!),
        rapid,
        () =>
        {
            _show(empty, true);
        },
        _source
    );
}

onMount(() =>
{
    syntaxScenarios.resolve({
        benchmark: _measureRendering,
        latency: _runLatency,
        lifecycle: _stressLifecycle,
        prepareVisibility: _prepareVisibility,
        verifyVisibility: _verifyVisibility
    });
    Object.assign(window, {
        verifySyntaxPresentation: _verifyPresentation,
        stressSyntaxLifecycle: _stressLifecycle,
        runSyntaxLatency: _runLatency,
        prepareSyntaxInteraction: _prepareInteraction,
        syntaxInteractionState: _interactionState,
        syntaxReady: true
    });
});
</script>

<div class="viewport-frame">
    <ScrollArea bind:viewport viewportClass="syntax-viewport">
        <div class="preview-inset">
            <Preview
                scope="benchmark"
                {content}
                viewport={viewport ?? null}
                busy={false}
                revision={viewRevision}
                onRead={_read}
                onLanguageChange={(_document: TextDocument, language: string | null) =>
                {
                    reportedLanguage = language;
                }}
            />
        </div>
    </ScrollArea>
</div>

<style>
.preview-inset { padding: 20px; }
.viewport-frame { width: fit-content; max-width: calc(100vw - 48px); margin: 24px; }
:global(.syntax-viewport) { width: 560px; max-width: calc(100vw - 48px); height: min(500px, calc(100vh - 48px)); flex: none; background: var(--surface-window); color: var(--text-primary); }
</style>
