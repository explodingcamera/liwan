import { defineConfig } from "tsdown";

export default defineConfig({
	entry: ["./src/index.ts", "./src/express.ts", "./src/hono.ts"],
	outDir: "dist",
	format: "esm",
	platform: "neutral",
	target: "es2022",
	minify: true,
	dts: true,
	outputOptions: { entryFileNames: "[name].js" },
});
