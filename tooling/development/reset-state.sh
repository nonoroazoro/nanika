#!/bin/sh

set -eu

if [ "$(uname -s)" != "Darwin" ]; then
    echo "Nanika development state reset currently supports macOS and Windows." >&2
    exit 1
fi

if pgrep -x nanika-desktop >/dev/null 2>&1; then
    echo "Stop the running Nanika development application before resetting its state." >&2
    exit 1
fi

user_home=${HOME:?HOME must be set}
product_data="$user_home/Library/Application Support/com.nanika.nanika"
webview_data="$user_home/Library/WebKit/nanika-desktop"
webview_cache="$user_home/Library/Caches/nanika-desktop"

case "$product_data:$webview_data:$webview_cache" in
    "$user_home/Library/Application Support/com.nanika.nanika:$user_home/Library/WebKit/nanika-desktop:$user_home/Library/Caches/nanika-desktop") ;;
    *)
        echo "Refusing to remove unexpected development paths." >&2
        exit 1
        ;;
esac

rm -rf -- "$product_data" "$webview_data" "$webview_cache"
