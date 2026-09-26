// bun add hono liwan-api
import { Hono } from "hono";
import { getConnInfo } from "hono/bun";
import { createClient } from "liwan-api";
import { honoMiddleware } from "liwan-api/hono";

const analytics = createClient({
	endpoint: "https://analytics.example.com",
	apiKey: Bun.env.LIWAN_API_KEY ?? "",
});

const app = new Hono();
app.use("*", honoMiddleware(analytics, "docs", { peerIp: (context) => getConnInfo(context).remote.address }));

app.get("/", (context) => context.text("Hello"));
const server = Bun.serve({ fetch: app.fetch, port: 3000 });

process.once("SIGTERM", () => {
	server.stop();
	void analytics.close().catch(console.error);
});
