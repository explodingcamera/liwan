import { defineConfig } from "tsdown";

export default defineConfig({
	entry: ["./script.ts"],
	outDir: ".",
	clean: false,
	format: "esm",
	platform: "browser",
	target: ["chrome129", "edge128", "firefox130", "safari18", "ios18"],
	minify: true,
	dts: true,
	outputOptions: { entryFileNames: "[name].js" },
});
