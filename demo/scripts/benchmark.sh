#!/usr/bin/env bash
set -e
cargo run --release --features runtime-benchmarks --bin node -- benchmark --chain=dev --steps=50 --repeat=20 "--pallet=*" "--extrinsic=*" --execution=wasm --wasm-execution=compiled --heap-pages=4096 --template=./templates/runtime-weight-template.hbs --output=./runtime/src/weights/
