<script lang="ts">
const { value, label, options, onChange }: {
    value: string;
    label: string;
    options: { value: string; label: string; }[];
    onChange: (value: string) => void;
} = $props();
const uid = $props.id();
let root = $state<HTMLDivElement>();
let trigger = $state<HTMLButtonElement>();
let open = $state(false);
let above = $state(false);
let highlighted = $state(0);
let prefix = "";
let typedAt = 0;
const selected = $derived(options.find(option => option.value === value));

function expand(): void
{
    highlighted = Math.max(0, options.findIndex(option => option.value === value));
    const bounds = trigger?.getBoundingClientRect();
    above = bounds !== undefined && innerHeight - bounds.bottom < options.length * 32 + 16
        && bounds.top > options.length * 32 + 16;
    prefix = "";
    open = true;
}

function choose(index: number): void
{
    const option = options[index];
    if (option)
    {
        onChange(option.value);
    }
    open = false;
    trigger?.focus();
}

function keydown(event: KeyboardEvent): void
{
    if (event.key === "Tab")
    {
        open = false;
        return;
    }
    if (event.key === "Escape")
    {
        if (open)
        {
            event.preventDefault();
            event.stopPropagation();
            open = false;
        }
        return;
    }
    if (["Enter", " ", "ArrowDown", "ArrowUp", "Home", "End"].includes(event.key))
    {
        event.preventDefault();
        if (!open)
        {
            expand();
        }
        else if (event.key === "Enter" || event.key === " ")
        {
            choose(highlighted);
            return;
        }
        else if (event.key === "ArrowDown" || event.key === "ArrowUp")
        {
            highlighted = (highlighted + (event.key === "ArrowDown" ? 1 : -1) + options.length) % options.length;
        }
        if (event.key === "Home" || event.key === "End")
        {
            highlighted = event.key === "Home" ? 0 : options.length - 1;
        }
    }
    else if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey && !event.isComposing)
    {
        event.preventDefault();
        if (!open)
        {
            expand();
        }
        const now = performance.now();
        prefix = (now - typedAt < 700 ? prefix : "") + event.key.toLowerCase();
        typedAt = now;
        const index = options.findIndex(option => option.label.toLowerCase().startsWith(prefix));
        if (index >= 0)
        {
            highlighted = index;
        }
    }
}
</script>

<svelte:window
    onpointerdown={event =>
    {
        if (open && !event.composedPath().includes(root as EventTarget))
        {
            open = false;
        }
    }}
    onblur={() =>
    {
        open = false;
    }}
    onresize={() =>
    {
        open = false;
    }}
/>
<div
    class="select"
    bind:this={root}
    onfocusout={event =>
    {
        if (!root?.contains(event.relatedTarget as Node | null))
        {
            open = false;
        }
    }}
>
    <button
        bind:this={trigger}
        type="button"
        class="trigger"
        role="combobox"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={`${uid}-options`}
        aria-activedescendant={open ? `${uid}-${highlighted}` : undefined}
        onclick={() =>
        {
            if (open)
            {
                open = false;
            }
            else
            {
                expand();
            }
        }}
        onkeydown={keydown}
    >
        <span>{selected?.label ?? value}</span>
        <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            aria-hidden="true"
        >
            <path d="m7 10 5 5 5-5" />
        </svg>
    </button>
    {#if open}
        <div class="options" class:above id={`${uid}-options`} role="listbox" aria-label={label}>
            {#each options as option, index (option.value)}
                <button
                    type="button"
                    role="option"
                    id={`${uid}-${index}`}
                    tabindex="-1"
                    aria-selected={option.value === value}
                    class:highlighted={index === highlighted}
                    onpointermove={() =>
                    {
                        highlighted = index;
                    }}
                    onpointerdown={event => event.preventDefault()}
                    onclick={() => choose(index)}
                >
                    <span>{option.label}</span>
                    {#if option.value === value}<svg
                            width="14"
                            height="14"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.8"
                            aria-hidden="true"
                        >
                            <path d="m5 12 4 4L19 6" />
                        </svg>{/if}
                </button>
            {/each}
        </div>
    {/if}
</div>

<style>
.select { position: relative; min-width: 112px; }
.trigger { display: flex; justify-content: space-between; gap: 16px; width: 100%; min-height: 32px; padding: 5px 9px; border: 1px solid var(--border-subtle); border-radius: 6px; background: var(--surface-window); font-size: 12px; font-weight: 400; }
.trigger:hover:not(:disabled), .trigger[aria-expanded="true"] { border-color: var(--border-window); background: var(--surface-hovered); }
.trigger svg { color: var(--text-secondary); flex-shrink: 0; }
.options { position: absolute; z-index: 10; top: calc(100% + 5px); right: 0; width: 100%; padding: 4px; border: 1px solid var(--border-window); border-radius: 8px; background: var(--surface-form); box-shadow: 0 6px 18px rgb(0 0 0 / 14%); }
.options.above { top: auto; bottom: calc(100% + 5px); }
.options button { width: 100%; justify-content: space-between; gap: 24px; padding: 6px 8px; border: 0; border-radius: 4px; font-size: 12px; font-weight: 400; text-align: left; }
.options button.highlighted { background: var(--surface-selected); }
.options svg { color: var(--accent); flex-shrink: 0; }
.trigger:focus-visible { border-color: var(--border-window); background: var(--surface-hovered); }
</style>
