/**
 * Authoritative values after completion; null effective state means application is unconfirmed.
 */
export interface SettingsWriteResult<T>
{
    values: T;
    saved: T;
    effective: T | null;
    error: string | null;
}
