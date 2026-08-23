Nanika is a native, keyboard-driven capability host.

Keep Nanika and the bundled extension executables in the same directory. User
configuration and generated data are stored outside the application directory.

Windows: run Nanika.exe.
macOS: move Nanika.app to Applications and open it normally.

Manage local external extensions while Nanika is stopped:

    nanika-cli install <package.nanika>
    nanika-cli update <package.nanika>
    nanika-cli enable <extension-id>
    nanika-cli disable <extension-id>
    nanika-cli remove <extension-id>

Export bounded local diagnostics while Nanika is running or stopped:

    nanika-cli diagnostics <output.zip>
