#!/bin/bash

set -e

rustup target add x86_64-unknown-linux-musl
cargo build --release --workspace

mkdir -p ./bin
cp target/release/pherowar ./bin/
cp target/release/player ./bin/