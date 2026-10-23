extension-name = Scripts
setting-script-roots-title = Folders
setting-script-roots-description = Find PowerShell, Python, JavaScript, and shell scripts in these folders.
group-search-locations = Search locations

action-run = Run

error-apply-configuration = Script settings could not be applied. Review the details and try again.
error-invalid-configuration = Script settings are invalid. Review the settings and correct the reported value.

error-read-catalog = The script catalog could not be read. Review the details and try again.
error-script-unavailable = This script is unavailable. Review the details and check the script file before trying again.
error-invalid-host-response = The host returned an unexpected response to the script action. Check whether it completed before trying again.
error-discovery = Scripts could not be discovered. Review the details and check the configured folders before trying again.
error-unsupported-protocol = Scripts does not support the requested extension protocol.
error-not-initialized = Scripts must finish initializing before handling requests.
error-unsupported-message = Scripts received an unsupported request.

error-host-service = { $outcome ->
    [cancelled] The script action was cancelled before it was accepted.
    [rejected] The host did not accept the script action. Review the details before trying again.
   *[failed] The script action reported a failure. Review the details before trying again.
    [unconfirmed] Nanika could not confirm whether the script was launched. Check before running it again to avoid duplicate work.
    }
