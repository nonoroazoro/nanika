extension-name = Calculator

action-copy-result = Copy result

error-unsupported-protocol = Calculator does not support the requested extension protocol.
error-not-initialized = Calculator must finish initializing before handling requests.
error-cancelled = The calculation was cancelled.
error-unknown-action = This calculation result or action is no longer available. Calculate the expression again.
error-unsupported-message = Calculator received an unsupported request.

error-host-service = { $outcome ->
    [cancelled] Copying the calculation result was cancelled before it was accepted.
    [rejected] The host did not accept the request to copy the calculation result. Review the details before trying again.
   *[failed] Copying the calculation result reported a failure. Review the details before trying again.
    [unconfirmed] Nanika could not confirm whether the calculation result was copied. Check the clipboard before trying again.
    }
