export interface StatusBarEntry
{
    id: string;
    title: string;
    icon?: "app" | { src: string; };
    iconOnly?: boolean;
    menu?: {
        controls: string;
        expanded: boolean;
    };
    interactive?: boolean;
    disabled?: boolean;
    destructive?: boolean;
    keys?: string[];
    ariaShortcut?: string;
    pending?: {
        active: boolean;
        title: string;
    };
}
