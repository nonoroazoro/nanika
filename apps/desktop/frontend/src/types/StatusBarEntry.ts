export interface StatusBarEntry
{
    id: string;
    title: string;
    icon?: "app";
    iconOnly?: boolean;
    menu?: {
        controls: string;
        expanded: boolean;
    };
    interactive?: boolean;
    disabled?: boolean;
    destructive?: boolean;
    confirmation?: {
        active: boolean;
        title: string;
    };
    keys?: string[];
    ariaShortcut?: string;
    pending?: {
        active: boolean;
        title: string;
    };
}
