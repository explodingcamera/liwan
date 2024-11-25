import path from "node:path";
import react from "@astrojs/react";
import { defineConfig } from "astro/config";
import license from "rollup-plugin-license";
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
		plugins: [
			license({
				thirdParty: {
					allow: "(MIT OR Apache-2.0 OR ISC OR BSD-3-Clause OR 0BSD OR CC0-1.0 OR Unlicense)",
					output: {
						file: path.join(dirname, "../", "data", "licenses-npm.json"),
						template: (dependencies) => JSON.stringify(dependencies),
					},
				},
			}),
		],
	},
	integrations: [react()],
	redirects: {
		"/settings": "/settings/projects",
	},
});
