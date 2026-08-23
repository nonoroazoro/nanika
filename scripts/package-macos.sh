#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
target=${NANIKA_MACOS_TARGET:-aarch64-apple-darwin}
version=${NANIKA_VERSION:-$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$repository_root/Cargo.toml" | head -n 1)}
sign_identity=${NANIKA_SIGN_IDENTITY:-}
notary_profile=${NANIKA_NOTARY_PROFILE:-}
case "$version" in
    ''|*[!0-9A-Za-z.+-]*)
        printf '%s\n' "Version contains characters that are unsafe for an artifact name." >&2
        exit 1
        ;;
esac
case "$target" in
    aarch64-apple-darwin|x86_64-apple-darwin) ;;
    *)
        printf '%s\n' "Unsupported macOS release target: $target" >&2
        exit 1
        ;;
esac
artifact_name="nanika-$version-macos-${target%%-*}"
dist_root="$repository_root/dist"
stage_root="$dist_root/$artifact_name"
app_root="$stage_root/Nanika.app"
macos_root="$app_root/Contents/MacOS"
resources_root="$app_root/Contents/Resources"
archive_path="$dist_root/$artifact_name.zip"
binary_root="$repository_root/target/$target/release"
case "$stage_root" in
    "$dist_root"/nanika-*) ;;
    *)
        printf '%s\n' "Release staging path escaped the dist directory." >&2
        exit 1
        ;;
esac
case "$archive_path" in
    "$dist_root"/nanika-*.zip) ;;
    *)
        printf '%s\n' "Release archive path escaped the dist directory." >&2
        exit 1
        ;;
esac

cargo build --release --locked --target "$target" \
    -p nanika-host \
    -p nanika-cli \
    -p nanika-extension-application \
    -p nanika-extension-command \
    -p nanika-extension-script \
    -p nanika-extension-calculator \
    -p nanika-extension-clipboard

rm -rf "$stage_root"
rm -f "$archive_path" "$archive_path.sha256"
mkdir -p "$macos_root" "$resources_root"
sed "s/@VERSION@/$version/g" "$repository_root/packaging/macos/Info.plist" > "$app_root/Contents/Info.plist"
cp "$binary_root/nanika-host" "$macos_root/Nanika"
cp "$binary_root/nanika-cli" "$macos_root/nanika-cli"
for extension in application command script calculator clipboard; do
    cp "$binary_root/nanika-extension-$extension" "$macos_root/nanika-extension-$extension"
done
cp "$repository_root/LICENSE" "$resources_root/LICENSE"
cp "$repository_root/packaging/README.txt" "$resources_root/README.txt"
chmod 755 "$macos_root"/*

if [ -n "$sign_identity" ]; then
    for executable in "$macos_root"/nanika-extension-* "$macos_root/nanika-cli" "$macos_root/Nanika"; do
        codesign --force --options runtime --timestamp --sign "$sign_identity" "$executable"
    done
    codesign --force --options runtime --timestamp --sign "$sign_identity" "$app_root"
    codesign --verify --deep --strict --verbose=2 "$app_root"
else
    printf '%s\n' "Warning: NANIKA_SIGN_IDENTITY is unset; creating an unsigned local artifact." >&2
fi

ditto -c -k --keepParent "$app_root" "$archive_path"

if [ -n "$notary_profile" ]; then
    if [ -z "$sign_identity" ]; then
        printf '%s\n' "NANIKA_NOTARY_PROFILE requires NANIKA_SIGN_IDENTITY." >&2
        exit 1
    fi
    xcrun notarytool submit "$archive_path" --keychain-profile "$notary_profile" --wait
    xcrun stapler staple "$app_root"
    xcrun stapler validate "$app_root"
    rm -f "$archive_path"
    ditto -c -k --keepParent "$app_root" "$archive_path"
fi

(cd "$dist_root" && shasum -a 256 "$(basename "$archive_path")" > "$(basename "$archive_path").sha256")
printf '%s\n' "$archive_path"
printf '%s\n' "$archive_path.sha256"
