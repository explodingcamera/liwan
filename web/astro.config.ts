import { defineConfig } from "astro/config";
import react from "@astrojs/react";
const proxy = {
	"/api": {
		target: "http://localhost:8080",
		changeOrigin: true,
		cookieDomainRewrite: "localhost:4321",
	},
};

// https://astro.build/config
export default defineConfig({
	vite: {
		server: { proxy },
		preview: { proxy },
	},
	integrations: [react()],
});
