#!/usr/bin/env bash
set -euo pipefail

rm -rf dist

cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
cargo build --release --target aarch64-apple-darwin

mkdir dist
mkdir dist/headers

cp crates/embedded-ffi/include/embedded_ffi.h dist/headers/
cat > dist/headers/module.modulemap <<'EOF'
module EmbeddedFFI {
  header "embedded_ffi.h"
  export *
}
EOF
cp target/aarch64-apple-ios-sim/release/libembedded_ffi.a dist/libembedded_ffi_ios_sim.a

lipo -create \
  target/aarch64-apple-darwin/release/libembedded_ffi.a \
  -output dist/libembedded_ffi_macos.a

xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libembedded_ffi.a -headers dist/headers \
  -library dist/libembedded_ffi_ios_sim.a -headers dist/headers \
  -library dist/libembedded_ffi_macos.a -headers dist/headers \
  -output dist/EmbeddedFFI.xcframework
