<script lang="ts">
import { onDestroy } from "svelte";

const { paths, maximum, label, titleId, description, onPick, onChange }: {
    paths: string[];
    maximum: number;
    label: string;
    titleId: string;
    description?: string | null;
    onPick: () => Promise<string | null>;
    onChange: (paths: string[]) => void;
} = $props();
let picking = $state(false);
let error = $state<string | null>(null);
let active = true;
onDestroy(() =>
{
    active = false;
});

async function add(): Promise<void>
{
    if (picking || paths.length >= maximum)
    {
        return;
    }
    picking = true;
    error = null;
    try
    {
        const path = await onPick();
        // Closing or switching the form must not apply a late picker result.
        if (active && path !== null && !paths.includes(path))
        {
            onChange([...paths, path]);
        }
    }
    catch (cause)
    {
        if (active)
        {
            console.error("Folder could not be added", cause);
            error = "Folder could not be added. Try again.";
        }
    }
    finally
    {
        if (active)
        {
            picking = false;
        }
    }
}
</script>

<div class="directories" role="group" aria-label={label}>
    <div class="directory-toolbar">
        <div class="copy">
            <h2 id={titleId}>{label}</h2>
            {#if description}<p>{description}</p>{/if}
        </div>
        <button
            type="button"
            class="add-folder"
            disabled={picking || paths.length >= maximum}
            onclick={() =>
            {
                void add();
            }}
        >
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                aria-hidden="true"
            >
                <path d="M12 5v14M5 12h14" />
            </svg>
            {picking ? "Choosing folder…" : "Add folder"}
        </button>
    </div>
    {#if paths.length > 0}<div class="directory-items">
            {#each paths as path, index (index)}
                <div class="directory">
                    <svg
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.6"
                        aria-hidden="true"
                    >
                        <path d="M3 7V5a2 2 0 0 1 2-2h5l3 4h6a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z" />
                    </svg>
                    <span class="path" title={path}>{path}</span>
                    <button
                        type="button"
                        class="remove-folder"
                        aria-label={`Remove folder ${path}`}
                        title="Remove folder"
                        disabled={picking}
                        onclick={() => onChange(paths.filter((_, position) => position !== index))}
                    >
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.6"
                            aria-hidden="true"
                        >
                            <circle cx="12" cy="12" r="8.5" />
                            <path d="M8 12h8" />
                        </svg>
                    </button>
                </div>
            {/each}
        </div>{/if}

    {#if error}<p role="alert">{error}</p>{/if}
</div>

<style>
.directories { display: grid; gap: 12px; }
.directory-toolbar { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
.copy { min-width: 0; }
h2 { margin: 0; font-size: 13px; font-weight: 500; }
.copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: 12px; line-height: 1.5; }
.add-folder { flex-shrink: 0; min-height: 32px; padding: 5px 10px; border: 1px solid var(--border-window); border-radius: 6px; background: var(--surface-form); font-size: 12px; }
.add-folder:hover:not(:disabled) { border-color: var(--border-window); background: var(--surface-raised); }
.directory-items { display: grid; gap: 8px; max-height: 320px; overflow-y: auto; scrollbar-gutter: stable; }
.directory-items { border-top: 1px solid var(--border-subtle); padding-top: 12px; }
.directory { display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
.directory > svg { flex-shrink: 0; color: var(--text-secondary); }
.path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
.directory button { flex-shrink: 0; }
.remove-folder { width: 32px; height: 32px; padding: 7px; border-radius: 6px; color: var(--text-danger); }
.remove-folder:hover:not(:disabled) { background: var(--surface-danger-hover); color: var(--text-danger); }
.remove-folder:focus-visible { background: var(--surface-danger-hover); }
p[role="alert"] { color: var(--text-danger); margin: 0; overflow-wrap: anywhere; }
</style>
