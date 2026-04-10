#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$ROOT_DIR/WooshhNotifier.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"

mkdir -p "$MACOS_DIR"
cp "$ROOT_DIR/Info.plist" "$CONTENTS_DIR/Info.plist"

RUST_TARGET="${WOOSHH_NOTIFIER_TARGET:-}"
case "$RUST_TARGET" in
  x86_64-apple-darwin)
    SWIFT_TARGET="x86_64-apple-macos12.0"
    ;;
  aarch64-apple-darwin)
    SWIFT_TARGET="arm64-apple-macos12.0"
    ;;
  "")
    SWIFT_TARGET=""
    ;;
  *)
    echo "Unsupported notifier target: $RUST_TARGET" >&2
    exit 1
    ;;
esac

if [[ -n "$SWIFT_TARGET" ]]; then
  xcrun --sdk macosx swiftc -target "$SWIFT_TARGET" "$ROOT_DIR/main.swift" -o "$MACOS_DIR/wooshh-notifier"
else
  xcrun --sdk macosx swiftc "$ROOT_DIR/main.swift" -o "$MACOS_DIR/wooshh-notifier"
fi

# Ad-hoc sign so macOS can treat it as a valid app executable.
codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1 || true

echo "Built: $APP_DIR"
echo "Notifier binary: $MACOS_DIR/wooshh-notifier"
