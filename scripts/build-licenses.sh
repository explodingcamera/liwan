#!/usr/bin/env bash
cd "$(dirname "$0")" && cd ..
cargo about generate --format json -o ./data/licenses-cargo.json
