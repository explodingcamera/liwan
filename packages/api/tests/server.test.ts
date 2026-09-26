import { describe, expect, test } from "bun:test";

import {
	clientIpFromRequest,
	createClient,
	createClientIpResolver,
	fromExpressRequest,
	fromNodeRequest,
	fromWebRequest,
} from "../src/index";

describe("server client", () => {
	test("batches events and records timestamps immediately", async () => {
		const requests: RequestInit[] = [];
		const urls: (URL | RequestInfo)[] = [];
		const client = createClient({
			endpoint: "https://liwan.example?source=test#fragment",
			apiKey: "liw_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
			batchSize: 2,
			fetch: async (input, init) => {
				urls.push(input);
				requests.push(init ?? {});
				return new Response(null, { status: 202 });
			},
		});

		client.event("docs", "pageview", { url: "https://example.com/one" });
		client.event("docs", "pageview", { url: "https://example.com/two" });
		await client.flush();

		expect(requests).toHaveLength(1);
		expect(String(urls[0])).toBe("https://liwan.example/api/v1/events");
		const body = JSON.parse(String(requests[0].body));
		expect(body.events).toHaveLength(2);
		expect(body.entityId).toBe("docs");
		expect(body.events[0].createdAt).toBeString();
		expect((requests[0].headers as Record<string, string>).Authorization).toBe(
			"Bearer liw_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
		);
	});

	test("continues with queued events after a rejected batch", async () => {
		let requests = 0;
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			batchSize: 1,
			flushInterval: 1,
			fetch: async () => new Response(null, { status: ++requests === 1 ? 400 : 202 }),
		});

		client.event("docs", "first", { url: "https://example.com/one" });
		client.event("docs", "second", { url: "https://example.com/two" });
		await expect(client.flush()).rejects.toThrow("status 400");
		await new Promise((resolve) => setTimeout(resolve, 10));

		expect(requests).toBe(2);
		await client.close();
	});

	test("rejects invalid timing options", () => {
		expect(() => createClient({ endpoint: "https://liwan.example", apiKey: "key", flushInterval: Number.NaN })).toThrow(
			"flushInterval must be positive",
		);
		expect(() =>
			createClient({ endpoint: "https://liwan.example", apiKey: "key", requestTimeout: Number.POSITIVE_INFINITY }),
		).toThrow("requestTimeout must be positive");
	});

	test("retries temporary failures", async () => {
		let requests = 0;
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			fetch: async () =>
				new Response(null, {
					status: ++requests === 1 ? 503 : 202,
					headers: { "Retry-After": "0.001" },
				}),
		});

		client.event("docs", "pageview", { url: "https://example.com" });
		await client.flush();
		expect(requests).toBe(2);
	});

	test("honors the retry limit", async () => {
		let requests = 0;
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			maxRetries: 0,
			fetch: async () => {
				requests++;
				return new Response(null, { status: 503 });
			},
		});
		client.event("docs", "pageview", { url: "https://example.com" });
		await expect(client.flush()).rejects.toThrow("status 503");
		expect(requests).toBe(1);
	});

	test("reports queue overflow synchronously", () => {
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			batchSize: 1,
			queueCapacity: 1,
			fetch: () => new Promise(() => {}),
		});
		client.event("docs", "pageview", { url: "https://example.com/one" });
		expect(() => client.event("docs", "pageview", { url: "https://example.com/two" })).toThrow("queue is full");
	});

	test("separates batches by entity", async () => {
		const bodies: unknown[] = [];
		const client = createClient({
			endpoint: "https://liwan.example",
			apiKey: "key",
			fetch: async (_input, init) => {
				bodies.push(JSON.parse(String(init?.body)));
				return new Response(null, { status: 202 });
			},
		});

		client.event("docs", "pageview", { url: "https://example.com/docs" });
		client.event("shop", "pageview", { url: "https://example.com/shop" });
		await client.flush();

		expect(bodies).toHaveLength(2);
		expect(bodies).toMatchObject([{ entityId: "docs" }, { entityId: "shop" }]);
	});

	test("maps Express request metadata", () => {
		const metadata = fromExpressRequest({
			originalUrl: "/docs?source=test",
			protocol: "https",
			ip: "203.0.113.1",
			get: (name) => ({ host: "example.com", referer: "https://search.example", "user-agent": "test" })[name],
		});
		expect(metadata.url).toBe("https://example.com/docs?source=test");
		expect(metadata.ip).toBe("203.0.113.1");
	});

	test("resolves Node client IP through trusted proxies", () => {
		const request = {
			headers: { "x-forwarded-for": "198.51.100.5, 10.0.0.1" },
			socket: { remoteAddress: "10.0.0.2" },
		};
		expect(clientIpFromRequest(request)).toBe("10.0.0.2");
		expect(
			clientIpFromRequest(request, {
				sources: ["x-forwarded-for"],
				trustedProxies: ["10.0.0.2"],
			}),
		).toBe("10.0.0.1");
		expect(
			clientIpFromRequest(request, {
				sources: ["x-forwarded-for"],
				trustedProxies: ["10.0.0.0/8"],
			}),
		).toBe("198.51.100.5");
		expect(
			clientIpFromRequest(
				{ ...request, headers: { "x-forwarded-for": "not-an-ip" } },
				{ sources: ["x-forwarded-for"], trustedProxies: ["*"] },
			),
		).toBe("10.0.0.2");
		expect(
			clientIpFromRequest(
				{ ...request, headers: { "cf-connecting-ip": "203.0.113.1" } },
				{ sources: ["cloudflare"], trustedProxies: ["10.0.0.2"] },
			),
		).toBe("203.0.113.1");
		expect(clientIpFromRequest({ headers: {}, socket: { remoteAddress: "2001:db8::1" } })).toBe("2001:db8::1");
		expect(clientIpFromRequest({ headers: {}, socket: { remoteAddress: "not-an-ip" } })).toBeUndefined();
		const resolve = createClientIpResolver({ sources: ["forwarded", "cloudfront"], trustedProxies: ["10.0.0.0/8"] });
		expect(resolve(new Headers({ forwarded: 'for="203.0.113.10";proto=https, for=10.0.0.1' }), "10.0.0.2")).toBe(
			"203.0.113.10",
		);
		expect(resolve(new Headers({ "cloudfront-viewer-address": "[2001:db8::1]:443" }), "10.0.0.2")).toBe("2001:db8::1");
		expect(resolve(new Headers({ "cloudfront-viewer-address": "[2001:db8::1]:443" }), "192.0.2.1")).toBe("192.0.2.1");
	});

	test("maps Node request metadata", () => {
		const request = {
			url: "/docs?source=test",
			headers: {
				host: "untrusted.example",
				referer: "https://search.example/",
				"user-agent": "test-agent",
				"x-forwarded-for": "203.0.113.10",
			},
			socket: { remoteAddress: "10.0.0.2" },
		};
		expect(fromNodeRequest(request, { origin: "https://example.com" })).toEqual({
			url: "https://example.com/docs?source=test",
			referrer: "https://search.example/",
			userAgent: "test-agent",
			ip: "10.0.0.2",
		});
		expect(
			fromNodeRequest(request, {
				origin: "https://example.com",
				clientIp: { sources: ["x-forwarded-for"], trustedProxies: ["10.0.0.2"] },
			}).ip,
		).toBe("203.0.113.10");
	});

	test("maps Web request metadata with an explicit IP", () => {
		const request = new Request("https://example.com/docs?source=test", {
			headers: { referer: "https://search.example/", "user-agent": "test-agent", "x-forwarded-for": "untrusted" },
		});
		expect(fromWebRequest(request)).toEqual({
			url: "https://example.com/docs?source=test",
			referrer: "https://search.example/",
			userAgent: "test-agent",
			ip: undefined,
		});
		expect(fromWebRequest(request, "203.0.113.10").ip).toBe("203.0.113.10");
	});
});
