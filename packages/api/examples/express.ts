import { createClient } from "@liwan.dev/api";
import { expressMiddleware } from "@liwan.dev/api/express";
import express from "express";

const analytics = createClient({
	endpoint: "https://analytics.example.com",
	apiKey: process.env.LIWAN_API_KEY ?? "",
});

const app = express();
// Trust only a reverse proxy connecting over loopback. Use your proxy's address or subnet otherwise.
app.set("trust proxy", "loopback");
app.use(expressMiddleware(analytics, "my-entity-id", { exclude: ["/health"] }));

app.get("/", (_request, response) => response.send("Hello"));
const server = app.listen(3000);

process.once("SIGTERM", () => {
	server.close(() => {
		void analytics.close().catch(console.error);
	});
});
