/**
 * Keeps a setting key paired with its value type at the write boundary.
 */
export type SettingsChange<T extends object> = {
    [K in keyof T]-?: { key: K; value: T[K]; };
}[keyof T];
