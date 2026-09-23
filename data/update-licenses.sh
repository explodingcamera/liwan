#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit
cargo about generate --format json -o ./data/licenses-cargo.json
