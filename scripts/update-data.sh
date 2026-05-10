#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(dirname "$0")"
ROOT_DIR="${SCRIPT_DIR}/.."
DATA_DIR="${ROOT_DIR}/data"

UA_REGEXES_URL="https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml"
SPAMMERS_URL="https://raw.githubusercontent.com/matomo-org/referrer-spam-list/refs/heads/master/spammers.txt"

for tool in curl yq zstd; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'missing required tool: %s\n' "$tool" >&2
        exit 1
    fi
done

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

curl -fsSL "$UA_REGEXES_URL" -o "$tmp_dir/regexes.yaml"
yq '.' "$tmp_dir/regexes.yaml" | zstd --quiet --force -o "$DATA_DIR/ua_regexes.json.zstd"

curl -fsSL "$SPAMMERS_URL" | tr -d '\r' | zstd --quiet --force -o "$DATA_DIR/spammers.txt.zstd"

printf 'updated %s and %s\n' "$DATA_DIR/ua_regexes.json.zstd" "$DATA_DIR/spammers.txt.zstd"
