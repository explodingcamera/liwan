import path from "node:path";
import react from "@astrojs/react";
import { defineConfig } from "astro/config";
import license from "rollup-plugin-license";
import type { AstroIntegration } from "astro";
const dirname = path.dirname(new URL(import.meta.url).pathname);

const proxy = {
	"/api": {
		target: "http://localhost:9042",
		changeOrigin: true,
		cookieDomainRewrite: "localhost:4321",
	},
};

function setPrerender(): AstroIntegration {
	let isDev = false;
	return {
		name: "set-prerender",
		hooks: {
			"astro:config:setup": ({ command }) => {
				isDev = command === "dev";
			},
			"astro:route:setup": ({ route }) => {
				if (isDev && route.component.endsWith("/pages/p/[...project].astro")) {
					route.prerender = false;
				}
			},
		},
	};
}

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
				// biome-ignore lint/suspicious/noExplicitAny: wrong type
			}) as any,
		],
	},
	integrations: [react(), setPrerender()],
	redirects: {
		"/settings": "/settings/projects",
	},
});
