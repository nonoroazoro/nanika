export interface NativeState
{
    pid: number;
    identity: string;
    platform: string;
    webviewVersion: string;
    windows: Record<string, { focused: boolean; pickerOpen: boolean; visible: boolean; }>;
}
