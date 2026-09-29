<script lang="ts">
import appIcon from "../../../shell/icons/icon.svg";
import { settingsAnchor } from "./anchor";

const { version }: { version: string; } = $props();
</script>

<div class="about-page" id={settingsAnchor("about", { kind: "page" })}>
    <h1>About</h1>
    <div class="about-content">
        <div class="identity">
            <img class="app-icon" src={appIcon} width="96" height="96" alt="" draggable="false" />
            <h2>Nanika</h2>
            <p>A home for your apps, commands, and extensions.</p>
        </div>
        <dl class="app-information" aria-label="Application information">
            <div class="information-row">
                <dt>Version</dt>
                <dd>{version}</dd>
            </div>
        </dl>
    </div>
</div>

<style>
.about-page { display: flex; flex-direction: column; min-height: 100%; max-width: 52rem; margin: 0 auto; padding: var(--space-4) var(--space-6) var(--space-6); }
h1 { margin: 0; font-size: 20px; line-height: 26px; font-weight: var(--settings-heading-weight); }
.about-content { display: flex; flex: 1; flex-direction: column; align-items: center; justify-content: center; gap: 32px; padding: 32px 0 64px; }
.identity { display: flex; flex-direction: column; align-items: center; text-align: center; }
.app-icon { display: block; margin-bottom: var(--space-4); }
h2 { margin: 0; color: var(--text-primary); font-size: 28px; line-height: 36px; font-weight: 600; letter-spacing: -0.02em; }
p { max-width: 18rem; margin: var(--space-2) 0 0; color: var(--text-secondary); font-size: var(--settings-label-size); line-height: var(--settings-label-line-height); }
.app-information { width: min(100%, 20rem); margin: 0; border: 1px solid var(--border-subtle); border-radius: var(--radius-row); background: var(--surface-form); }
.information-row { display: flex; align-items: baseline; justify-content: space-between; gap: var(--space-4); padding: var(--space-3) var(--space-4); font-size: var(--settings-label-size); line-height: var(--settings-label-line-height); }
dt { color: var(--text-secondary); }
dd { margin: 0; color: var(--text-primary); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
@media (width < 800px) { .about-page { padding-right: var(--space-4); padding-left: var(--space-4); } }
</style>
