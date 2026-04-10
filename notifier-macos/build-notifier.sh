#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$ROOT_DIR/WooshhNotifier.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"

mkdir -p "$MACOS_DIR"
cp "$ROOT_DIR/Info.plist" "$CONTENTS_DIR/Info.plist"

swiftc "$ROOT_DIR/main.swift" -o "$MACOS_DIR/wooshh-notifier"

# Ad-hoc sign so macOS can treat it as a valid app executable.
codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1 || true

echo "Built: $APP_DIR"
echo "Notifier binary: $MACOS_DIR/wooshh-notifier"
