import type { ListItem } from "../../src/generated/ListItem";
import type { View } from "../../src/generated/View";

const content = "const original = '内容';";

export const textReadFailure: View = {
    kind: "detail",
    detail: {
        title: "Retained text",
        content: {
            kind: "text",
            value: "Visible source. ".repeat(1024),
            syntax_highlighting: false,
            document: {
                id: "read-error",
                revision: "1",
                utf16Length: 16385,
                lineCount: 1,
                maxLineLength: 16385,
                totalChunks: 2
            }
        },
        metadata: [],
        actions: []
    }
};

const summaries: Array<Pick<ListItem, "icon" | "title">> = [
    { title: { message: "content-text" }, icon: "text" },
    { title: { message: "entry-image", args: { width: 1920, height: 1080 } }, icon: "image" },
    { title: { message: "entry-files", args: { name: "报告.txt", remaining: 2 } }, icon: "files" }
];

const items: ListItem[] = Array.from({ length: 80 }, (_, index) => ({
    id: `entry-${index}`,
    ...summaries[index] ?? { title: `Record ${index}`, icon: "text" },
    subtitle: { message: "metadata-content-type" },
    actions: []
}));

export const localizedList: View = {
    kind: "list",
    list: {
        title: { message: "extension-name" },
        search_placeholder: { message: "view-search-placeholder" },
        search_text: "",
        empty_title: { message: "view-empty-title" },
        empty_description: { message: "view-empty-description" },
        layout: "split",
        collection_id: "localized-list",
        selection: items[0] ? { index: 0, item: items[0] } : null,
        sections: [{
            id: "history",
            title: { message: "extension-name" },
            offset: 0,
            total: 80,
            items
        }],
        detail: {
            title: { message: "content-text" },
            content: {
                kind: "text",
                value: content,
                document: {
                    id: "localized-document",
                    revision: "1",
                    utf16Length: content.length,
                    lineCount: 1,
                    maxLineLength: content.length,
                    totalChunks: 1
                },
                syntax_highlighting: false
            },
            metadata: [
                { title: { message: "metadata-content-type" }, value: { message: "content-text" } },
                { title: { message: "metadata-size" }, value: { bytes: 1536 } }
            ],
            actions: []
        },
        filter: {
            id: "contentType",
            selected_value: "all",
            options: [
                { value: "all", title: { message: "filter-all" } },
                { value: "text", title: { message: "content-text" } }
            ]
        }
    }
};

export const localizedFiles: View = structuredClone(localizedList);
if (localizedFiles.kind === "list" && localizedFiles.list.detail)
{
    localizedFiles.list.detail.title = null;
    localizedFiles.list.detail.content = {
        kind: "files",
        files: Array.from({ length: 7 }, (_, index) => ({
            path: `/Example/用户文件夹/name ${index}.txt`,
            name: `name ${index}.txt`,
            icon: null
        }))
    };
}

export const longLabels: View = structuredClone(localizedList);
if (longLabels.kind === "list" && longLabels.list.filter && longLabels.list.detail)
{
    const title = {
        fallback: "ExtendedClipboardContentMetadata".repeat(3),
        translations: {
            en: "ExtendedClipboardContentMetadata".repeat(3),
            "zh-Hans": "用于跨语言布局验证的内容类型与详细信息".repeat(2)
        }
    };
    for (const option of longLabels.list.filter.options)
    {
        option.title = title;
    }
    for (const entry of longLabels.list.detail.metadata)
    {
        entry.title = title;
    }
}
