#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TMP_DIR='/tmp/cosmox-plugin-example'

cargo generate niniconi/cosmox-plugin-template --name cosmox-plugin-example --destination /tmp

cd "$REPO_ROOT"
cargo build --release -p cosmox-plugin-packager
cd "$TMP_DIR"
cargo build --release
cp "$REPO_ROOT/target/release/cosmox-plugin-packager" "$TMP_DIR"
"$TMP_DIR"/cosmox-plugin-packager pack --input "$TMP_DIR" --release --output "$TMP_DIR"/out


rm -rf "$SCRIPT_DIR/cosmox_plugin_example"

cp -a "$TMP_DIR/out/release/cosmox-plugin-example/build/" "$SCRIPT_DIR/cosmox_plugin_example"

rm -rf "$TMP_DIR"
