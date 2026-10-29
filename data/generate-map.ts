#!/usr/bin/env -S bun -i
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { GeometryCollection, Topology } from "topojson-specification";

import { unzipSync } from "fflate@0.8.3";
import mapshaper from "mapshaper@0.7.101";

const args = process.argv.slice(2);
if (args.length > 1 || args[0] === "--help" || args[0] === "-h") {
	console.log("Usage: ./data/generate-map.ts [output.json]\nSet SIMPLIFY to change retention (default: 8%).");
	process.exit(args.length > 1 ? 1 : 0);
}

const output = resolve(args[0] ?? fileURLToPath(new URL("./geo.json", import.meta.url)));
const simplify = process.env.SIMPLIFY ?? "8%";

const response = await fetch("https://naturalearth.s3.amazonaws.com/50m_cultural/ne_50m_admin_0_countries.zip");
if (!response.ok) throw new Error(`Natural Earth download failed: ${response.status}`);
const files = unzipSync(new Uint8Array(await response.arrayBuffer()));
const source = "ne_50m_admin_0_countries";
const version = new TextDecoder().decode(files[`${source}.VERSION.txt`]).trim();

const generated = await mapshaper.applyCommands(
	[
		"-i",
		`${source}.shp`,
		"-filter",
		'ISO_A2_EH != "AQ"',
		"-simplify",
		"weighted",
		simplify,
		"-filter",
		"remove-empty",
		"-proj",
		"+proj=robin +datum=WGS84",
		"-rename-fields",
		"name=NAME,iso=ISO_A2_EH",
		"-filter-fields",
		"name,iso",
		"-rename-layers",
		"geo",
		"-o",
		"geo.json",
		"format=topojson",
		"width=800",
		"height=500",
		"margin=0",
		"quantization=10000",
	],
	Object.fromEntries(Object.entries(files).map(([name, data]) => [name, Buffer.from(data)])),
);

const names = new Map(
	(await Bun.file(new URL("./countries.txt", import.meta.url)).text())
		.trim()
		.split(/\r?\n/)
		.map((line) => {
			const [name, iso] = line.split("=");
			return [iso, name];
		}),
);
const geo = JSON.parse(generated["geo.json"]) as Topology<{
	geo: GeometryCollection<{ name: string; iso: string }>;
}>;
geo.objects.geo.geometries = geo.objects.geo.geometries.filter((geometry) => geometry.type);
if (geo.transform) {
	let minX = Infinity;
	let minY = Infinity;
	let maxX = -Infinity;
	let maxY = -Infinity;
	for (const arc of geo.arcs) {
		let x = 0;
		let y = 0;
		for (const [dx, dy] of arc) {
			x += dx;
			y += dy;
			minX = Math.min(minX, x);
			minY = Math.min(minY, y);
			maxX = Math.max(maxX, x);
			maxY = Math.max(maxY, y);
		}
	}
	geo.transform.translate = [
		400 - ((minX + maxX) * geo.transform.scale[0]) / 2,
		250 - ((minY + maxY) * geo.transform.scale[1]) / 2,
	];
}
for (const { properties } of geo.objects.geo.geometries) {
	if (!properties) continue;
	if (!/^[A-Z]{2}$/.test(properties.iso)) properties.iso = "";
	properties.name = names.get(properties.iso) ?? properties.name;
}
const bytes = await Bun.write(output, JSON.stringify(geo));
console.log(`Generated ${output} from Natural Earth ${version} (${bytes} bytes)`);
