import { getConnInfo } from "@hono/bun";
import { createClient } from "@liwan.dev/api";
import { honoMiddleware } from "@liwan.dev/api/hono";
import { Hono } from "hono";

const analytics = createClient({
	endpoint: "https://analytics.example.com",
	apiKey: Bun.env.LIWAN_API_KEY ?? "",
});

const app = new Hono();
app.use(
	"*",
	honoMiddleware(analytics, "my-entity-id", {
		peerIp: (context) => getConnInfo(context).remote.address,
		clientIp: {
			sources: ["x-forwarded-for"],
			// Trust only a reverse proxy connecting over loopback. Use your proxy's address or subnet otherwise.
			trustedProxies: ["127.0.0.1", "::1"],
		},
	}),
);

app.get("/", (context) => context.text("Hello"));
const server = Bun.serve({ fetch: app.fetch, port: 3000 });

process.once("SIGTERM", () => {
	server.stop();
	void analytics.close().catch(console.error);
});
