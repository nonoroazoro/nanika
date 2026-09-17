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
product_data="$user_home/Library/Application Support/Nanika"
product_cache="$user_home/Library/Caches/Nanika"
# WKWebView owns this browser state outside Nanika's application-managed roots.
webview_state="$user_home/Library/WebKit/nanika-desktop"
webview_cache="$user_home/Library/Caches/nanika-desktop"

case "$product_data:$product_cache:$webview_state:$webview_cache" in
    "$user_home/Library/Application Support/Nanika:$user_home/Library/Caches/Nanika:$user_home/Library/WebKit/nanika-desktop:$user_home/Library/Caches/nanika-desktop") ;;
    *)
        echo "Refusing to remove unexpected development paths." >&2
        exit 1
        ;;
esac

rm -rf -- "$product_data" "$product_cache" "$webview_state" "$webview_cache"
