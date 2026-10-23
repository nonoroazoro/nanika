extension-name = System

action-lock-title = Lock Screen
action-sleep-title = Sleep
action-displays-title = Turn Off Displays
action-logout-title = { $platform ->
    [windows] Sign out
   *[macos] Log Out
    }
action-logout-confirmation-title = { $platform ->
    [windows] Sign out?
   *[macos] Log Out?
    }
action-logout-confirmation-description = Your current session will end and open apps will close. Save your work before continuing.
action-restart-title = Restart
action-restart-confirmation-title = Restart?
action-restart-confirmation-description = Your computer will restart and open apps will close. Save your work before continuing.
action-shutdown-title = Shut Down
action-shutdown-confirmation-title = Shut Down?
action-shutdown-confirmation-description = Your computer will shut down and open apps will close. Save your work before continuing.
action-trash-open-title = { $platform ->
    [windows] Open Recycle Bin
   *[macos] Open Trash
    }
action-trash-empty-title = { $platform ->
    [windows] Empty Recycle Bin
   *[macos] Empty Trash
    }
action-trash-empty-confirmation-title = { $platform ->
    [windows] Empty Recycle Bin?
   *[macos] Empty Trash?
    }
action-trash-empty-confirmation-description = { $platform ->
    [windows] All items in the Recycle Bin will be permanently deleted. This cannot be undone.
   *[macos] All items in the Trash will be permanently deleted. This cannot be undone.
    }

command-lock-description = Lock your current session
command-sleep-description = Put the computer to sleep
command-displays-description = Put connected displays to sleep
command-logout-description = End your current user session
command-restart-description = Restart this computer
command-shutdown-description = Shut down this computer
command-trash-open-description = Open deleted items in the system file manager
command-trash-empty-description = { $platform ->
    [windows] Permanently delete all items in the Recycle Bin
   *[macos] Permanently delete all items in Trash
    }
keyword-delete = delete
keyword-display = display
keyword-empty-recycle-bin = empty recycle bin
keyword-lock = lock
keyword-logout = logout
keyword-monitor = monitor
keyword-power-off = power off
keyword-reboot = reboot
keyword-recycle-bin = recycle bin
keyword-restart = restart
keyword-screen = screen
keyword-shutdown = shutdown
keyword-sign-out = sign out
keyword-sleep = sleep
keyword-suspend = suspend
keyword-trash = trash
keyword-turn-off-display = turn off display

error-already-initialized = System is already initialized.
error-unsupported-protocol = System does not support the requested extension protocol.
error-not-initialized = System must finish initializing before handling requests.
error-busy = A system action is pending. Wait for it to finish before trying again.
error-unknown-action = This system action is unavailable.
error-invalid-response = The system service returned an unexpected response. Check whether the action completed before trying again.
error-unsupported-message = System received an unsupported request.

error-host-service = { $outcome ->
    [cancelled] The system action was cancelled before it was accepted.
    [rejected] The host did not accept the system action. Review the details before trying again.
   *[failed] The system service reported a failure. Review the details before trying again.
    [unconfirmed] Nanika could not confirm whether the system action was submitted. Check the system state before trying again.
    }
