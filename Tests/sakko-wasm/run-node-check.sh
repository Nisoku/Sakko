#!/usr/bin/env bash
# Check of the compiled `sakko-wasm` module under Node.

set -euo pipefail

cd "$(dirname "$0")/../.."

cargo build --release --target wasm32-unknown-unknown -p sakko-wasm

mkdir -p target/wasm-node
wasm-bindgen \
  target/wasm32-unknown-unknown/release/sakko_wasm.wasm \
  --target nodejs \
  --out-dir target/wasm-node

node Tests/sakko-wasm/node-check.cjs target/wasm-node