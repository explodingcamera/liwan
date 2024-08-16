#!/usr/bin/env bash

esbuild ../tracker/script.ts --minify --format=esm --target=chrome123,edge123,firefox124,safari17 --outfile=../tracker/script.min.js
