#!/usr/bin/env bash
cd "$(dirname "$0")" && cd ..
cargo zigbuild --release --target x86_64-unknown-linux-musl
rsync -avzP target/x86_64-unknown-linux-musl/release/liwan pegasus:~/.local/bin/liwan
