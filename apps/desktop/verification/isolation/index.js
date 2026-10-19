const allowed = new Set([
    "verification_report",
    "verification_progress",
    "verification_resources"
]);
window.__TAURI_ISOLATION_HOOK__ = message =>
{
    if (!allowed.has(message?.cmd) || !message.payload || typeof message.payload !== "object")
    {
        throw new Error("Blocked verification IPC");
    }
    return message;
};
