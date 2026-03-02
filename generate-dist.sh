#!/usr/bin/env bash
set -euo pipefail

PACKAGE="embedded-ffi"
LIB_NAME="libembedded_ffi.a"
DIST_DIR="dist"
HEADERS_DIR="$DIST_DIR/headers"

TARGETS=(
  aarch64-apple-ios
  aarch64-apple-ios-sim
  aarch64-apple-darwin
  aarch64-apple-tvos
  aarch64-apple-tvos-sim
  aarch64-apple-visionos
  aarch64-apple-visionos-sim
  aarch64-apple-watchos
  aarch64-apple-watchos-sim
)

rm -rf "$DIST_DIR"
mkdir -p "$HEADERS_DIR"

for target in "${TARGETS[@]}"; do
  cargo build --release -p "$PACKAGE" --target "$target"
done

cp crates/embedded-ffi/include/embedded_ffi.h "$HEADERS_DIR/"
cat > "$HEADERS_DIR/module.modulemap" <<'EOF'
module EmbeddedFFI {
  header "embedded_ffi.h"
  export *
}
EOF

xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-ios-sim/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-darwin/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-tvos/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-tvos-sim/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-visionos/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-visionos-sim/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-watchos/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -library "target/aarch64-apple-watchos-sim/release/$LIB_NAME" -headers "$HEADERS_DIR" \
  -output "$DIST_DIR/EmbeddedFFI.xcframework"
