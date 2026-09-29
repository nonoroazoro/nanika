<script lang="ts">
import PlusIcon from "../components/icons/PlusIcon.svelte";
import FolderIcon from "../components/icons/FolderIcon.svelte";
import CircleMinusIcon from "../components/icons/CircleMinusIcon.svelte";
import ScrollArea from "../components/ui/ScrollArea.svelte";
import Button from "../components/ui/Button.svelte";
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
        <Button
            variant="outline"
            class="add-folder"
            disabled={picking || paths.length >= maximum}
            onclick={() =>
            {
                void add();
            }}
        >
            <PlusIcon size={16} strokeWidth={1.6} />
            Add folder
        </Button>
    </div>
    {#if paths.length > 0}<ScrollArea style="max-height: 320px; flex: none;"><div class="directory-items">
                {#each paths as path, index (index)}
                    <div class="directory">
                        <FolderIcon size={18} strokeWidth={1.6} />
                        <span class="path" title={path}>{path}</span>
                        <Button
                            class="remove-folder"
                            aria-label={`Remove folder ${path}`}
                            title="Remove folder"
                            disabled={picking}
                            onclick={() => onChange(paths.filter((_, position) => position !== index))}
                        >
                            <CircleMinusIcon size={18} strokeWidth={1.6} />
                        </Button>
                    </div>
                {/each}
            </div></ScrollArea>{/if}

    {#if error}<p role="alert">{error}</p>{/if}
</div>

<style>
.directories { display: grid; gap: var(--space-2); }
.directory-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
.copy { min-width: 0; }
h2 { margin: 0; font-size: var(--settings-label-size); line-height: var(--settings-label-line-height); font-weight: 400; }
.copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); }
.directories :global(.add-folder) { flex-shrink: 0; }
.directory-items { display: grid; gap: var(--space-2); border-top: 1px solid var(--border-subtle); padding-top: var(--space-2); }
.directory { display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
.directory > :global(svg) { flex-shrink: 0; color: var(--text-secondary); }
.path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; -webkit-user-select: text; user-select: text; }
.directory :global(button) { flex-shrink: 0; }
.directories :global(.remove-folder) { width: 32px; height: 32px; padding: 7px; border-radius: var(--control-radius); color: var(--text-danger); }
.directories :global(.remove-folder:hover:not(:disabled)) { background: var(--surface-danger-hover); color: var(--text-danger); }
.directories :global(.remove-folder:focus-visible) { background: var(--surface-danger-hover); }
p[role="alert"] { color: var(--text-danger); margin: 0; overflow-wrap: anywhere; }
</style>
