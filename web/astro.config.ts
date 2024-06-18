import { defineConfig } from "astro/config";
import react from "@astrojs/react";
const proxy = {
  "/api": {
    target: "http://localhost:8008",
    changeOrigin: true,
    cookieDomainRewrite: "dawdle.space",
    ws: true
  }
};

// https://astro.build/config
export default defineConfig({
  vite: {
    server: {
      proxy
    },
    preview: {
      proxy
    }
  },
  integrations: [react()]
});