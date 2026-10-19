<script lang="ts">
import { onDestroy } from "svelte";
import { confirmAction } from "../ui/confirmation";
import { SvelteMap } from "svelte/reactivity";
import Button from "../components/ui/Button.svelte";
import type { SettingsAction } from "../generated/SettingsAction";
import { settingsAnchor } from "./anchor";

const { extensionId, iconUrl, actions, disabled, onInvoke, onError }: {
    extensionId: string;
    iconUrl: string;
    actions: SettingsAction[];
    disabled: boolean;
    onInvoke: (id: string, confirmed: boolean) => Promise<void>;
    onError: (message: string) => void;
} = $props();
let alive = true;
onDestroy(() =>
{
    alive = false;
});
const variants = { primary: "primary", secondary: "outline", destructive: "danger" } as const;
let invoking = false;
const groups = $derived.by(() =>
{
    const result = new SvelteMap<string, SettingsAction[]>();
    for (const item of actions)
    {
        const group = item.action.group ?? "Actions";
        const entries = result.get(group) ?? [];
        entries.push(item);
        result.set(group, entries);
    }
    return [...result.entries()];
});

async function _invoke(item: SettingsAction): Promise<void>
{
    if (disabled || invoking || !item.action.enabled)
    {
        return;
    }
    invoking = true;
    try
    {
        const confirmed = item.action.confirmation_title
            ? await confirmAction({
                title: item.action.confirmation_title,
                destructive: item.action.style === "destructive",
                label: item.action.title,
                message: item.action.confirmation_description ?? item.description,
                icon: item.action.icon ? { kind: "symbol", name: item.action.icon } : { kind: "image", url: iconUrl }
            })
            : false;
        if (
            !alive || disabled || !actions.includes(item) || !item.action.enabled
            || (item.action.confirmation_title && !confirmed)
        )
        {
            return;
        }
        await onInvoke(item.action.id, confirmed);
    }
    catch (error)
    {
        onError(String(error));
    }
    finally
    {
        invoking = false;
    }
}
</script>

{#each groups as [group, entries] (group)}
    <section class="action-group" aria-label={group}>
        <h2>{group}</h2>
        <div class="fields">
            {#each entries as item (item.action.id)}
                <section
                    class="property scalar settings-search-target"
                    id={settingsAnchor(extensionId, { kind: "action", key: item.action.id })}
                >
                    <div class="property-copy">
                        <h3>{item.action.title}</h3>
                        <p>{item.description}</p>
                    </div>
                    <div class="property-control">
                        <Button
                            size="action"
                            variant={variants[item.action.style]}
                            disabled={disabled || !item.action.enabled}
                            onclick={() => _invoke(item)}
                        >
                            {item.buttonLabel ?? item.action.title}
                        </Button>
                    </div>
                </section>
            {/each}
        </div>
    </section>
{/each}

<style>
.action-group { margin-top: var(--space-6); }
.action-group > h2 { margin: 0 0 var(--space-3); font-size: var(--font-meta); color: var(--text-secondary); font-weight: 600; }
</style>
