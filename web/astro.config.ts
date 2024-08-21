import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import license from "rollup-plugin-license";
import path from "node:path";
const dirname = path.dirname(new URL(import.meta.url).pathname);

const proxy = {
	"/api": {
		target: "http://localhost:9042",
		changeOrigin: true,
		cookieDomainRewrite: "localhost:4321",
	},
};

// https://astro.build/config
export default defineConfig({
	vite: {
		server: { proxy },
		preview: { proxy },
		// css: { transformer: "lightningcss" },
		plugins: [
			license({
				thirdParty: {
					allow: "(MIT OR Apache-2.0 OR ISC OR BSD-3-Clause OR 0BSD OR CC0-1.0 OR Unlicense)",
					output: {
						file: path.join(dirname, "../", "data", "licenses-npm.json"),
						template: (dependencies) => JSON.stringify(dependencies),
					},
				},
				// biome-ignore lint/suspicious/noExplicitAny: type is correct
			}) as any,
		],
	},
	integrations: [react()],
	redirects: {
		"/settings": "/settings/projects",
	},
});
