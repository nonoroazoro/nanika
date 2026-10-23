extension-name = Clipboard History
error-text-read = Clipboard text could not be loaded. Try again or reopen Clipboard History.
setting-clipboard-syntax-highlighting-title = Syntax highlighting
setting-clipboard-syntax-highlighting-description = Color recognized code and Markdown in text previews.
group-appearance = Appearance
setting-clipboard-max-entries-title = Maximum history entries
setting-clipboard-max-entries-description = Maximum number of clipboard history entries retained locally.
group-history-retention = History retention
setting-clipboard-max-age-days-title = Maximum age in days
setting-clipboard-max-age-days-description = Maximum age of clipboard history retained locally.
group-danger-zone = Danger zone
setting-clipboard-clear-description = Permanently delete all clipboard history. This cannot be undone.
setting-clipboard-clear-button-label = Clear

action-copy = Copy to Clipboard
action-clipboard-clear-title = Clear history
action-clipboard-clear-confirmation-title = Clear all history?
action-clipboard-clear-confirmation-description = All clipboard history will be permanently deleted. This cannot be undone.

view-history-description = Search and copy previous clipboard content.
category-clipboard = Clipboard
keyword-clipboard = clipboard
keyword-history = history
keyword-paste = paste

view-search-placeholder = Search clipboard history
view-empty-title = No clipboard history yet
view-empty-description = Copied content will appear here.
view-empty-filtered-title = No matching entries
view-empty-filtered-description = Try a different search or filter.
filter-all = All Types
content-text = Text
content-files = Files
content-images = Images
content-image = Image
metadata-content-type = Content type
metadata-size = Size

entry-image = Image { $width } × { $height }
entry-files = { $name } and { $remaining } more

error-clear-history = Clipboard history could not be cleared. Review the details before trying again.
error-action-unavailable = This clipboard action is no longer available. Reopen Clipboard History and try again.
error-open-history = Clipboard History could not be opened. Review the details and try again.

error-update-history = Clipboard History could not be updated. Your current view is retained. Try again or reopen it.
error-close-history = Clipboard History could not be closed. Try going back again.
error-history-changed = Clipboard History has changed. Reopen it before continuing.
error-history-interaction = This interaction is unavailable in Clipboard History. Reopen it and try again.
error-copied-monitor-failed = The item was copied, but clipboard tracking could not be completed. Check the clipboard before trying again.

error-apply-configuration = Clipboard History settings could not be applied. Review the details and try again.
error-invalid-configuration = Clipboard History settings are invalid. Review the settings and correct the reported value.

error-unsupported-protocol = Clipboard History does not support the requested extension protocol.
error-not-initialized = Clipboard History must finish initializing before handling requests.
error-unsupported-message = Clipboard History received an unsupported request.

error-host-service = { $outcome ->
    [cancelled] Copying the history item was cancelled before it was accepted.
    [rejected] The host did not accept the request to copy the history item. Review the details before trying again.
   *[failed] Copying the history item reported a failure. Review the details before trying again.
    [unconfirmed] Nanika could not confirm whether the history item was copied. Check the clipboard before trying again.
    }
