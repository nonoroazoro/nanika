<script lang="ts">
import { operationFailure } from "../localization/operation-errors";
import type { SettingsError } from "./SettingsError";
import type { LocalizedText } from "../generated/LocalizedText";
import type { ExtensionPresentation } from "../localization/ExtensionPresentation";
import { localization } from "../localization/state.svelte";
import { onDestroy } from "svelte";
import { confirmAction } from "../ui/confirmation";
import { SvelteMap } from "svelte/reactivity";
import Button from "../components/ui/Button.svelte";
import type { SettingsAction } from "../generated/SettingsAction";
import { settingsAnchor } from "./anchor";

const { extensionId, iconUrl, actions, disabled, onInvoke, onError, presentation }: {
    presentation: ExtensionPresentation;
    extensionId: string;
    iconUrl: string;
    actions: SettingsAction[];
    disabled: boolean;
    onInvoke: (id: string, confirmed: boolean) => Promise<void>;
    onError: (message: SettingsError) => void;
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
    const result = new SvelteMap<string, { title: LocalizedText | null; entries: SettingsAction[]; }>();
    for (const item of actions)
    {
        const group = item.action.group;
        const id = group?.id ?? "$actions";
        const entry = result.get(id) ?? { title: group?.title ?? null, entries: [] };
        entry.entries.push(item);
        result.set(id, entry);
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
        const prepared = presentation;
        const confirmed = item.action.confirmation_title
            ? await confirmAction({
                content: () => ({
                    title: prepared.actions.text(item.action.confirmation_title!, localization.locale),
                    label: prepared.actions.text(item.action.title, localization.locale),
                    message: item.action.confirmation_description
                        ? prepared.actions.text(item.action.confirmation_description, localization.locale)
                        : prepared.text(item.description, localization.locale)
                }),
                destructive: item.action.style === "destructive",
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
        onError(operationFailure(error, "invocationUnconfirmed"));
    }
    finally
    {
        invoking = false;
    }
}
</script>

{#each groups as [id, group] (id)}
    <section
        class="action-group"
        aria-label={group.title
        ? presentation.actions.text(group.title, localization.locale)
        : localization.text({ message: "actions-label" })}
    >
        <h2>
            {
                group.title
                ? presentation.actions.text(group.title, localization.locale)
                : localization.text({ message: "actions-label" })
            }
        </h2>
        <div class="fields">
            {#each group.entries as item (item.action.id)}
                <section
                    class="property scalar settings-search-target"
                    id={settingsAnchor(extensionId, { kind: "action", key: item.action.id })}
                >
                    <div class="property-copy">
                        <h3>{presentation.actions.text(item.action.title, localization.locale)}</h3>
                        <p>{presentation.text(item.description, localization.locale)}</p>
                    </div>
                    <div class="property-control">
                        <Button
                            size="action"
                            variant={variants[item.action.style]}
                            disabled={disabled || !item.action.enabled}
                            onclick={() => _invoke(item)}
                        >
                            {
                                item.buttonLabel
                                ? presentation.text(item.buttonLabel, localization.locale)
                                : presentation.actions.text(item.action.title, localization.locale)
                            }
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
