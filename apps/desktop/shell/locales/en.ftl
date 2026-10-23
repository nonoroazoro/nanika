settings-general = General
settings-launcher = Launcher
settings-open-launcher = Open launcher
settings-hide-on-blur = Hide when focus is lost
settings-appearance = Appearance
settings-theme = Theme
settings-theme-system = System
settings-theme-light = Light
settings-theme-dark = Dark
settings-language = Language
settings-language-system = Follow System
settings-startup = Startup
settings-launch-at-login = Launch at login
settings-launch-at-login-description = Launch Nanika automatically when you sign in.
settings-startup-approval = Turn this on to open System Settings and allow Nanika.
settings-startup-repair = Turn this on to repair launch at login.
settings-about = About
tray-open = Open Nanika
tray-quit = Quit
settings-window-title = Nanika Settings
native-add-folder = Add folder
settings-enable-extension = Enable { $name }
settings-enable-extension-title = Enable extension
extension-menu-label = { $name } menu
localization-load-failed = Translations could not load.
action-retry = Try again

settings-progress-units = { $label }: { $completed } of { $total }
settings-progress-updating = Updating { $label }

error-configuration-save-failed = The change could not be saved and was not applied.
error-configuration-applied-not-saved = The change took effect but could not be saved.
error-configuration-unavailable = The extension is unavailable; the change was not saved.
error-configuration-unconfirmed-not-saved = The change could not be confirmed and was not saved.
error-configuration-saved-not-applied = The change was saved but could not be applied.
notification-dismiss = Dismiss notification

validation-unknown-setting = Unknown setting.
validation-whole-number = Enter a whole number.
validation-integer-minimum = Enter { $value } or more.
validation-integer-maximum = Enter { $value } or less.
validation-integer-multiple = Enter a multiple of { $value }.
validation-integer-limit = Choose a finite or unlimited integer limit.
validation-text-required = Enter text.
validation-text-bytes = Enter at most { $value } UTF-8 bytes.
validation-boolean-required = Choose on or off.
validation-array-required = Enter a list.
validation-array-length = Enter at most { $value } entries.
validation-object-required = Enter an object.
validation-entry = Entry { $index }
validation-field = { $field }: { $error }
settings-limit-finite-label = { $label }: Use limit { $value }
settings-limit-finite-title = Use a numeric limit
settings-limit-unlimited-label = { $label }: Unlimited
settings-limit-unlimited-title = Unlimited
settings-array-entry-label = { $label }, entry { $index }
settings-array-remove-label = Remove { $label } entry { $index }
settings-array-remove = Remove
settings-array-add = Add entry
settings-array-show-more = Show more ({ $count })
settings-array-count = { $count } / { $maximum }
settings-object-include = Include
settings-object-include-label = Include { $field }
settings-object-field-label = { $label }, { $field }

settings-title = Settings
settings-navigation = Settings navigation
settings-content = Settings content
settings-loading = Loading settings…
settings-search = Search settings
settings-search-placeholder = Search
settings-search-clear = Clear search
settings-search-results = Settings search results
settings-sections = Settings sections
settings-search-empty = No settings found
settings-search-count = { $count ->
    [one] { $count } result
   *[other] { $count } results
    }
settings-extensions = Extensions
settings-about-description = A home for your apps, commands, and extensions.
settings-app-information = Application information
settings-version = Version
window-controls = Window controls
window-minimize = Minimize
window-maximize = Maximize
window-restore = Restore
window-close-settings = Close Settings
settings-folder-add = Add folder
settings-folder-remove = Remove folder
settings-shortcut-label = Open launcher shortcut
settings-shortcut-recording = Press keys…
settings-shortcut-modifiers = { $platform ->
    [macos] Use Control, Option, or Command with a key.
   *[windows] Use Ctrl, Alt, or the Windows key with another key.
    }
extension-state = { $state ->
    [disabled] Disabled
    [dormant] Ready on demand
    [starting] Starting…
    [ready] Running
    [stopping] Stopping…
   *[failed] Unavailable
    }
launcher-label = Nanika launcher
launcher-search = Search for apps and commands
launcher-results = Results
launcher-searching = Searching extensions…
launcher-searching-count = { $count ->
    [one] Searching { $count } extension…
   *[other] Searching { $count } extensions…
    }
launcher-empty = No results
search-empty-description = Try a different search.
launcher-menu = Nanika menu
launcher-unavailable = Search is unavailable.
launcher-display-failed = Search could not be displayed.
actions-label = Actions
action-cancel = Cancel
action-confirm = Confirm
view-label = Extension view
view-back = Back to previous view
view-search = Search this view
view-content-filter = Content filter
view-details = Details
view-items = Items
view-loading-items = Loading items…
view-items-load-failed = Items could not load.
view-text-content = Detail content
view-text-load-more = Load more text
view-files = Files
view-file-collection = File collection
view-file-collection-label = { $count ->
    [one] Collection of { $count } item
   *[other] Collection of { $count } items
    }
view-file-count = { $count ->
    [one] { $count } item
   *[other] { $count } items
    }
view-file-paths = { $count ->
    [one] Path
   *[other] Paths
    }
view-files-more = { $count ->
    [one] { $count } more file
   *[other] { $count } more files
    }

error-operation-extension-unavailable = The extension is no longer available. Reopen the view to try again.
error-operation-cancelled = The request was cancelled.
error-operation-extension-terminated = The extension stopped before replying. An accepted system action may still complete.
error-operation-extension-io = Communication with the extension failed. Reopen the view to try again.
error-operation-extension-protocol = The extension returned an invalid response. Reopen the view to try again.
error-operation-text-read-failed = More text could not be loaded. Try again.
error-operation-text-read-timeout = Loading more text timed out. Try again.
error-operation-text-source-changed = The text source changed. Wait for the view to refresh.
error-operation-invalid-text-read = The text request is invalid. Reopen the view to try again.
error-operation-session-unavailable = This window session is no longer available. Reopen Nanika.
error-operation-runtime-unavailable = Nanika is not ready to handle this request. Try again shortly.

error-operation-search-changed = Search results have changed. Select a current result and try again.
error-operation-action-unavailable = This action is no longer available. Reopen its menu or view and try again.
error-operation-operation-pending = Another operation is still running. Wait for it to finish before trying again.
error-operation-invocation-failed = The action could not be completed. Review the details before trying again.
error-operation-navigation-failed = The view could not be opened or closed. An accepted action may still have completed. Reopen Nanika to continue.
error-operation-view-operation-failed = The view could not be updated. Review the details and reopen it if needed.

error-operation-invocation-unconfirmed = The action outcome could not be confirmed. It may have completed. Check the result before trying again.
error-operation-settings-navigation-unsupported = This extension tried to open a Launcher view from Settings. Reopen the action in the Launcher.

error-operation-view-operation-unconfirmed = The view operation could not be confirmed. An accepted action may have completed. Reopen the view and check before trying again.
error-operation-view-changed = The view has changed. Use its current state or reopen it.
error-operation-view-interaction-unavailable = This interaction is unavailable in the current view. Reopen the view and try again.
error-operation-confirmation-changed = The view changed after confirmation. Review the current action and confirm again.
error-operation-navigation-depth-reached = Too many views are open. Go back before opening another view.
error-operation-cleanup-failed = Releasing the new view also failed:

error-operation-configuration-unconfirmed = No final configuration result was received.

error-operation-refresh-unconfirmed = The extension closed without confirming the catalog refresh.
error-operation-host-services-unavailable = A system service is unavailable. Some extension capabilities may not work.
error-operation-extension-storage-unavailable = Extension records could not be loaded. Check the details before retrying.
error-operation-extension-package-unavailable = An extension package could not be loaded. Check the details before retrying.
error-operation-extension-icon-unavailable = The extension icon could not be loaded.
error-operation-extension-configuration-unavailable = The extension settings could not be loaded. Check the details before retrying.

error-operation-extension-exited = The extension stopped unexpectedly.
error-operation-extension-stop-unconfirmed = The extension worker exited without confirming that it stopped.
error-operation-extension-still-stopping = The previous extension instance is still stopping. Wait for it to finish before enabling the extension.
error-operation-extension-registry-save-failed = The extension's enabled setting could not be saved. Its previous setting is unchanged.
error-operation-extension-lifecycle-unconfirmed = The extension operation ended without a confirmed result. Check its current state before trying again.
error-operation-extension-lifecycle-failed = The extension's enabled state could not be changed. Review the details and try again.
error-operation-catalog-retirement-failed = The extension's search results could not be withdrawn.
error-operation-application-shutting-down = Nanika is shutting down.
error-lifecycle-restarting = The extension failed. One automatic restart is in progress.
error-lifecycle-stopping = The restarted extension failed. Its instance is being stopped.
error-lifecycle-restart-failed = The automatic restart could not start. Disable and enable the extension to try again.
error-lifecycle-restart-limit = The extension failed again after one automatic restart. Automatic restart has stopped. Disable and enable the extension to try again.
error-lifecycle-recovery-stopped = Extension recovery stopped. Review the failures below.
error-lifecycle-original = Original failure
error-lifecycle-restarted = Restart failure
error-lifecycle-stop = Recovery failure

error-configuration-rejected = The change was not accepted and was not saved or applied.
error-configuration-unconfirmed = It is unclear whether the change was saved or applied. Check the current settings before trying again.
error-operation-configuration-changed = These settings have changed. Reopen the menu and try again.
error-operation-configuration-property-unavailable = This setting is unavailable here.
error-operation-invalid-configuration = This setting does not meet its requirements. Review the value before trying again.
error-operation-configuration-submission-failed = The settings operation could not start. Try again.

error-operation-runtime-startup-failed = Nanika could not start.
error-operation-built-in-inventory-unavailable = Nanika could not load its built-in extensions.
error-operation-host-configuration-unavailable = Nanika could not open its configuration.
error-operation-extension-registry-unavailable = Nanika could not read the extension registry.
error-operation-host-storage-unavailable = Nanika could not open or update its database.
error-operation-search-unavailable = Nanika could not start search.
error-operation-startup-diagnostic-unavailable = Startup diagnostics are unavailable.
launcher-startup-recovery = Review the diagnostics, resolve the reported issue, then restart Nanika.
launcher-show-diagnostics = Show diagnostics
launcher-hide-diagnostics = Hide diagnostics
launcher-loading-diagnostics = Loading diagnostics…
launcher-diagnostics = Startup diagnostics
launcher-no-diagnostics = No additional diagnostics were recorded.
error-operation-invalid-search-request = The search request is invalid. Edit the input and try again.
error-operation-search-query-too-long = The search query is too long. Shorten it and try again.
error-operation-search-submission-failed = Search could not be updated. Edit the input to try again.
error-operation-settings-search-failed = Settings could not be searched. Try again.
error-operation-settings-read-failed = Settings could not be read. Try again.
error-operation-settings-open-failed = The Settings window could not open. Try again.
error-operation-settings-ready-failed = The Settings window could not finish updating. Try again.
error-operation-window-drag-failed = The window could not be moved. Try dragging it again.
error-operation-window-minimize-failed = The window could not be minimized. Try again.
error-operation-window-resize-failed = The window size could not be changed. Try again.
error-operation-window-close-failed = The window could not be hidden. Try again.
error-operation-startup-read-failed = The launch-at-login status could not be read. Try again.
error-operation-startup-write-unconfirmed = The launch-at-login change could not be confirmed. Check its current status before trying again.
search-input-too-long = Search supports up to { $maximum } characters. Edit the input to continue.
error-operation-invalid-host-preferences = The launcher preferences are invalid. Check the values and try again.
error-operation-invalid-shortcut = The shortcut is invalid. Record a different shortcut.
error-operation-shortcut-registration-failed = The shortcut could not be registered. Check whether another app uses it.
error-operation-shortcut-release-failed = The previous shortcut could not be released. Check the current shortcut before retrying.
error-operation-shortcut-cleanup-failed = The new shortcut could not be released during recovery. It may still be active.
error-operation-preferences-save-failed = Launcher preferences could not be saved. Check the current settings before retrying.
error-operation-preferences-rollback-failed = The previous preferences could not be restored. Saved and active settings may differ.
error-operation-preferences-confirmation-failed = Saved preferences could not be confirmed. Check the current settings before retrying.
error-operation-preferences-unconfirmed = The preference change could not be confirmed. Check the current settings before retrying.
error-operation-native-language-update-failed = Preferences were saved, but native language updates failed. Try the language setting again.
error-operation-language-update-failed = The language could not be updated. Try again.
error-operation-directory-setting-unavailable = This setting no longer accepts folders. Reopen its settings and try again.
error-operation-directory-picker-failed = The folder picker could not complete. Try again.
error-operation-directory-unavailable = The selected folder is unavailable. Choose another folder.
error-operation-directory-path-invalid = The folder path is not supported. Choose another folder.
error-operation-shortcut-recording-failed = Shortcut recording could not be updated. Try again.
error-operation-session-open-failed = The launcher session could not open. Reopen the launcher and try again.
error-operation-search-acknowledgement-failed = Search updates could not be acknowledged. Reopen the launcher and try again.
error-operation-invalid-result-range = The requested result range is invalid. Reopen the launcher and try again.
error-operation-localization-busy = Too many translations are loading. Wait for them to finish, then try again.
error-operation-localization-scope-invalid = The translation request is invalid. Reopen the extension and try again.
error-operation-localization-resources-unavailable = The extension has no available translations. Check its package and try again.
error-operation-localization-package-changed = The translations belong to a different package. Reopen the extension and try again.
error-operation-localization-instance-retired = The extension instance for these translations has ended. Reopen the extension and try again.
