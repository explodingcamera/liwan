#!/usr/bin/env bash
cd "$(dirname "$0")"
esbuild ../tracker/script.ts --minify --format=esm --target=chrome123,edge123,firefox124,safari17 --outfile=../tracker/script.min.js
bunx tsc ../tracker/script.ts --target ESNext --module ESNext --declaration --emitDeclarationOnly --outFile ../tracker/script.d.ts
