import { expect, spyOn, test } from "bun:test";

const mockFetch = (requests: unknown[]) => {
	const preconnect = globalThis.fetch.preconnect;
	return spyOn(globalThis, "fetch").mockImplementation(
		Object.assign(
			async (_url: Parameters<typeof fetch>[0], init?: Parameters<typeof fetch>[1]) => {
				requests.push(JSON.parse(String(init?.body)));
				return new Response(null, { status: 200 });
			},
			{ preconnect },
		),
	);
};

test("global properties apply to server-side events", async () => {
	const requests: unknown[] = [];
	const fetchMock = mockFetch(requests);
	globalThis.liwanProperties = { plan: "pro" };

	try {
		const { event, setProperties } = await import("../src/script");
		await event("signup", { endpoint: "https://analytics.example/api/event", url: "https://example.com/" });
		setProperties({ plan: "team", source: "docs" });
		await event("signup", {
			endpoint: "https://analytics.example/api/event",
			url: "https://example.com/",
			properties: { plan: "enterprise", method: "email" },
		});
		setProperties({ plan: null });
		await event("signup", { endpoint: "https://analytics.example/api/event", url: "https://example.com/" });
		expect(requests).toMatchObject([
			{ properties: { plan: "pro" } },
			{ properties: { plan: "enterprise", method: "email", source: "docs" } },
			{ properties: { source: "docs" } },
		]);
		expect(requests[2]).not.toHaveProperty("properties.plan");
	} finally {
		fetchMock.mockRestore();
		delete globalThis.liwanProperties;
	}
});

test("event transformer receives normalized pageviews and custom events", async () => {
	const requests: unknown[] = [];
	const logs: string[] = [];
	const listeners = new Map<string, () => void>();
	const documentListeners = new Map<string, () => void>();
	const globals = ["window", "document", "location", "localStorage", "screen", "addEventListener", "__liwan_loaded"];
	const originalGlobals = globals.map((name) => Object.getOwnPropertyDescriptor(globalThis, name));
	const document = {
		referrer: "",
		visibilityState: "visible",
		querySelector: () => ({
			src: "https://analytics.example/script.js",
			getAttribute: (name: string) => (name === "data-entity" ? "example" : null),
		}),
		addEventListener: (name: string, listener: () => void) => documentListeners.set(name, listener),
	};
	const fetchMock = mockFetch(requests);
	const infoMock = spyOn(console, "info").mockImplementation((message: string) => logs.push(message));
	Object.assign(globalThis, {
		window: globalThis,
		location: new URL("https://example.com/start"),
		localStorage: { getItem: () => null },
		document,
		addEventListener: (name: string, listener: () => void) => listeners.set(name, listener),
		screen: { width: 1200, orientation: { type: "landscape-primary" } },
	});
	globalThis.liwanProperties = { plan: "pro" };
	let transformations = 0;
	globalThis.liwanTransformEvent = (payload) => {
		transformations++;
		const path = new URL(payload.url).pathname;
		return {
			...payload,
			name: path === "/renamed" ? "signup" : payload.name,
			properties: { ...payload.properties, path, event_type: payload.name },
		};
	};

	try {
		// Load a separate instance after the server-side test imports the module without a window.
		const browserScript = "../src/script.ts?browser-test";
		const { event, trackPageviews } = (await import(browserScript)) as typeof import("../src/script");
		expect(requests).toMatchObject([
			{
				name: "pageview",
				entity_id: "example",
				screen_width: "lg",
				properties: { plan: "pro", path: "/start", event_type: "pageview" },
			},
		]);
		trackPageviews();
		expect(requests).toHaveLength(1);
		expect(logs).toEqual(["[liwan]: Pageview tracking already started"]);
		await event("signup");
		expect(requests).toMatchObject([
			{ name: "pageview", properties: { plan: "pro", path: "/start" } },
			{ name: "signup", properties: { plan: "pro", path: "/start", event_type: "signup" } },
		]);
		await event("pageview", { url: "https://example.com/manual?secret=123#frag", exit: false });
		expect(requests[2]).toMatchObject({
			name: "pageview",
			url: "https://example.com/manual",
			properties: { plan: "pro", path: "/manual" },
		});
		Object.assign(globalThis, { location: new URL("https://example.com/next") });
		listeners.get("popstate")?.();
		expect(requests[3]).toMatchObject({ name: "pageview", properties: { plan: "pro", path: "/next" } });
		await new Promise((resolve) => setTimeout(resolve, 0));
		document.visibilityState = "hidden";
		documentListeners.get("visibilitychange")?.();
		expect(requests[4]).toMatchObject({ name: "pageview", url: "https://example.com/next", exit: true });
		expect(requests[4]).not.toHaveProperty("properties");
		expect(transformations).toBe(4);
		document.visibilityState = "visible";
		documentListeners.get("visibilitychange")?.();
		await event("pageview", { url: "https://example.com/renamed" });
		expect(requests[5]).toMatchObject({ name: "signup" });
		document.visibilityState = "hidden";
		documentListeners.get("visibilitychange")?.();
		expect(requests).toHaveLength(6);
	} finally {
		fetchMock.mockRestore();
		infoMock.mockRestore();
		delete globalThis.liwanProperties;
		delete globalThis.liwanTransformEvent;
		for (const [index, name] of globals.entries()) {
			const descriptor = originalGlobals[index];
			if (descriptor) Object.defineProperty(globalThis, name, descriptor);
			else Reflect.deleteProperty(globalThis, name);
		}
	}
});
