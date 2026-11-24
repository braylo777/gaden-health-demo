#!/bin/sh
# Build web/gaden.wasm from ../wasm. No npm. Needs: rustup target add wasm32-unknown-unknown
set -e
cd "$(dirname "$0")/../wasm"
cargo build --release --target wasm32-unknown-unknown
cp "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/gaden_wasm.wasm" ../web/gaden.wasm
ls -l ../web/gaden.wasm
