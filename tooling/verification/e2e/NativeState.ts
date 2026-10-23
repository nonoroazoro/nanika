export interface NativeState
{
    pid: number;
    identity: string;
    platform: string;
    webviewVersion: string;
    trayLabels: [string, string];
    windows: Record<string, { focused: boolean; pickerOpen: boolean; title: string; visible: boolean; }>;
}
