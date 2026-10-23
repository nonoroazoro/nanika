import { actionFields } from "./action-fields";

import type { PresentationField } from "./PresentationField";
import type { DetailView } from "../generated/DetailView";
import type { ListItem } from "../generated/ListItem";
import type { View } from "../generated/View";

/**
 * Prepare every label in a publication, including its independently delivered selection.
 * Domain documents, file names, and user search text retain their original content.
 * @param view Validated host view snapshot
 */
export function viewFields(view: View): PresentationField[]
{
    if (view.kind === "detail")
    {
        return _detailFields(view.detail);
    }
    const list = view.list;
    const fields: PresentationField[] = [
        { value: list.title, maximum: 256 },
        { value: list.search_placeholder, maximum: 256, allowEmpty: true },
        { value: list.empty_title, maximum: 256 },
        { value: list.empty_description, maximum: 512, allowEmpty: true }
    ];
    for (const section of list.sections)
    {
        if (section.title !== null)
        {
            fields.push({ value: section.title, maximum: 256 });
        }
        for (const item of section.items)
        {
            fields.push(..._itemFields(item));
        }
    }
    if (list.selection)
    {
        fields.push(..._itemFields(list.selection.item));
    }
    if (list.detail)
    {
        fields.push(..._detailFields(list.detail));
    }
    for (const option of list.filter?.options ?? [])
    {
        fields.push({ value: option.title, maximum: 128 });
    }
    return fields;
}

function _itemFields(item: ListItem): PresentationField[]
{
    const fields = [{ value: item.title, maximum: 512 }, ...actionFields(item.actions)];
    if (item.subtitle !== null)
    {
        fields.push({ value: item.subtitle, maximum: 512, allowEmpty: true });
    }
    return fields;
}

function _detailFields(detail: DetailView): PresentationField[]
{
    const fields = actionFields(detail.actions);
    if (detail.title !== null)
    {
        fields.push({ value: detail.title, maximum: 512, allowEmpty: true });
    }
    if (detail.content.kind === "image")
    {
        fields.push({ value: detail.content.alternative_text, maximum: 512 });
    }
    for (const entry of detail.metadata)
    {
        fields.push(
            { value: entry.title, maximum: 256 },
            { value: entry.value, maximum: 2048, allowEmpty: true }
        );
    }
    return fields;
}
