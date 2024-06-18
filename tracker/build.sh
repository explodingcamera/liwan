#!/usr/bin/env bash

esbuild script.ts --minify --format=esm --target=chrome123,edge123,firefox124,safari17 --outfile=script.min.js
