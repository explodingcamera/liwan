import { describe, expect, test } from "bun:test";
import express from "express";
import { Hono } from "hono";

import { expressMiddleware } from "../src/express";
import { honoMiddleware } from "../src/hono";
import { createClient } from "../src/index";

describe("tracking middleware", () => {
	test("tracks matching Hono requests without blocking responses", async () => {
		const batches: unknown[] = [];
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			fetch: async (_input, init) => {
				batches.push(JSON.parse(String(init?.body)));
				return new Response(null, { status: 202 });
			},
		});
		const app = new Hono();
		app.use(
			"*",
			honoMiddleware(client, "docs", {
				exclude: ["/health"],
				peerIp: () => "10.0.0.2",
				clientIp: { sources: ["cloudflare"], trustedProxies: ["10.0.0.2"] },
			}),
		);
		app.get("/docs", (context) => context.text("ok"));
		app.get("/health", (context) => context.text("ok"));
		app.post("/docs", (context) => context.text("ok"));

		const response = await app.request("https://example.com/docs", {
			headers: { "cf-connecting-ip": "203.0.113.1" },
		});
		expect(response.status).toBe(200);
		await app.request("https://example.com/health");
		await app.request("https://example.com/docs", { method: "POST" });
		await client.close();

		expect(batches).toMatchObject([
			{
				entityId: "docs",
				events: [{ name: "pageview", url: "https://example.com/docs", ip: "203.0.113.1" }],
			},
		]);
	});

	test("tracks Express requests using the resolved IP", async () => {
		const batches: unknown[] = [];
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			fetch: async (_input, init) => {
				batches.push(JSON.parse(String(init?.body)));
				return new Response(null, { status: 202 });
			},
		});
		const app = express();
		app.use(expressMiddleware(client, "docs", { include: ["/docs"] }));
		app.get("/docs", (_request, response) => response.send("ok"));
		app.get("/health", (_request, response) => response.send("ok"));
		const server = app.listen(0);
		try {
			const address = server.address();
			if (!address || typeof address === "string") throw new Error("no listening address");
			const origin = `http://127.0.0.1:${address.port}`;
			expect((await fetch(`${origin}/docs`)).status).toBe(200);
			await fetch(`${origin}/health`);
			await client.close();
			expect(batches).toMatchObject([
				{
					entityId: "docs",
					events: [{ name: "pageview", url: `${origin}/docs` }],
				},
			]);
			const event = (batches[0] as { events: { ip: string }[] }).events[0];
			expect(["127.0.0.1", "::ffff:127.0.0.1"]).toContain(event.ip);
		} finally {
			server.close();
		}
	});
});
