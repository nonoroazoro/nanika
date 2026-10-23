import { LocalizationState } from "./LocalizationState.svelte";
import { hostResources } from "../generated/host-resources";

export const localization = new LocalizationState(hostResources);
