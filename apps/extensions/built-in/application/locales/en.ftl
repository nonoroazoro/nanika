extension-name = Applications
setting-application-builtin-macos-system-applications-title = System applications
setting-application-builtin-macos-system-applications-description = Search system application folders.
group-search-locations = Search locations
setting-application-builtin-macos-user-applications-title = User applications
setting-application-builtin-macos-user-applications-description = Search applications in your user folder.
setting-application-builtin-windows-packaged-title = Packaged applications
setting-application-builtin-windows-packaged-description = Search registered packaged applications available to your user.
setting-application-builtin-windows-system-programs-title = System Start Menu
setting-application-builtin-windows-system-programs-description = Search the shared Windows Start Menu.
setting-application-builtin-windows-scoop-global-title = Global Scoop shims
setting-application-builtin-windows-scoop-global-description = Search active shims from the machine-wide Scoop installation.
setting-application-builtin-windows-scoop-user-title = User Scoop shims
setting-application-builtin-windows-scoop-user-description = Search active shims from your Scoop installation.
setting-application-builtin-windows-user-programs-title = User Start Menu
setting-application-builtin-windows-user-programs-description = Search your Windows Start Menu.
setting-application-roots-title = Folders
setting-application-roots-description = Search these folders in addition to your system applications.

action-open = Open
action-reveal-shortcut = Open shortcut location
action-reveal-finder = Show in Finder
action-reveal-file = Open file location
action-reveal-target = Open target location
group-location = Location

candidate-kind = Application

progress-finding-sources = Finding application sources
progress-scanning-sources = Scanning application sources
progress-updating-index = Updating application index

error-apply-configuration = Application settings could not be applied. Review the details and try again.
error-invalid-configuration = Application settings are invalid. Review the settings and correct the reported value.
error-refresh-catalog = The application catalog could not be refreshed. Review the details and try again.

error-read-catalog = The application catalog could not be read. Review the details and try again.
error-action-unavailable = This application action is no longer available. Search for the application again.
error-invalid-host-response = The host returned an unexpected response to the application action. Check whether it completed before trying again.
error-refresh-cancelled = The application scan was cancelled.
error-unsupported-protocol = Applications does not support the requested extension protocol.
error-not-initialized = Applications must finish initializing before handling requests.
error-unsupported-message = Applications received an unsupported request.

error-host-service = { $outcome ->
    [cancelled] The application action was cancelled before it was accepted.
    [rejected] The host did not accept the application action. Review the details before trying again.
   *[failed] The application action reported a failure. Review the details before trying again.
    [unconfirmed] Nanika could not confirm the application action outcome. Check the application or file manager before trying again.
    }
