<script lang="ts">
interface Props
{
    keys: string[];
    success?: boolean;
}

const { keys, success = false }: Props = $props();

function needsOpticalLift(key: string): boolean
{
    return key === "," || key === "." || key === ";" || key === ":";
}
</script>

<span class="shortcut-keys" class:success aria-hidden="true">
    {#each keys as key, index (`${index}:${key}`)}
        {#if index > 0}<span class="separator">+</span>{/if}<kbd><span
                class="key-label"
                class:optical-lift={needsOpticalLift(key)}
            >{key}</span></kbd>
    {/each}
</span>

<style>
.shortcut-keys { display: inline-flex; min-width: 0; align-items: center; gap: var(--space-1); }
kbd { display: inline-flex; min-width: 1.25rem; height: 1.25rem; align-items: center; justify-content: center; padding: 0 0.3125rem; border: 0; border-radius: 0.25rem; background: var(--surface-hovered); color: var(--text-secondary); font: inherit; font-size: 0.75rem; font-weight: 500; line-height: 1; transition: background-color var(--motion-control) var(--motion-ease), color var(--motion-control) var(--motion-ease); }
.key-label { display: block; line-height: 1; }
.key-label.optical-lift { transform: translateY(-0.125em); }
.separator { display: inline-flex; width: 0.5rem; height: 1.25rem; align-items: center; justify-content: center; color: var(--text-tertiary); font-size: 0.6875rem; font-weight: 500; line-height: 1; transition: color var(--motion-control) var(--motion-ease); }
.shortcut-keys.success kbd { background: var(--surface-success); color: var(--text-success); }
.shortcut-keys.success .separator { color: var(--text-success); }
</style>
