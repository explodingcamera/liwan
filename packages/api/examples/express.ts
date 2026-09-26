import express from "express";
import { createClient } from "liwan-api";
import { expressMiddleware } from "liwan-api/express";

const analytics = createClient({
	endpoint: "https://analytics.example.com",
	apiKey: process.env.LIWAN_API_KEY ?? "",
});

const app = express();
// Configure Express's "trust proxy" setting if the app runs behind a trusted reverse proxy.
app.use(expressMiddleware(analytics, "docs", { exclude: ["/health"] }));

app.get("/", (_request, response) => response.send("Hello"));
const server = app.listen(3000);

process.once("SIGTERM", () => {
	server.close(() => {
		void analytics.close().catch(console.error);
	});
});
