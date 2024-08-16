#!/usr/bin/env bash
cd "$(dirname "$0")"
cargo about generate --format json -o ../data/licenses-cargo.json
