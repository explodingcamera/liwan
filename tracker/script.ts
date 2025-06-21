declare global {
	interface Window {
		__liwan_loaded?: boolean;
		// biome-ignore lint/suspicious/noExplicitAny: no
		navigation?: any;
	}
}

type Payload = {
	name: string;
	url: string;
	referrer?: string;
	utm?: { campaign?: string; content?: string; medium?: string; source?: string; term?: string };
};

export type EventOptions = {
	/**
	 * The URL of the page where the event occurred.
	 *
	 * If not provided, the current page URL with hash and search parameters removed will be used.
	 */
	url?: string;

	/**
	 * The referrer of the page where the event occurred.
	 *
	 * If not provided, `document.referrer` will be used if available.
	 */
	referrer?: string;

	/**
	 * The API endpoint to send the event to.
	 *
	 * If not provided, either the `data-api` attribute or the url where the script is loaded from will be used.
	 * Required in server-side environments.
	 */
	endpoint?: string;

	/**
	 * The entity that the event is associated with.
	 *
	 * If not provided, the `data-entity` attribute will be used.
	 * Required for custom events.
	 */
	entity?: string;
};

let scriptEl: HTMLScriptElement | null = null;
let endpoint: string | null = null;
let entity: string | null = null;
let referrer: string | null = null;
const noWindow = typeof window === "undefined";

if (typeof document !== "undefined") {
	scriptEl = document.querySelector(`script[src^="${import.meta.url}"]`);
	endpoint = scriptEl?.getAttribute("data-api") || (scriptEl && new URL(scriptEl.src).origin + "/api/event");
	entity = scriptEl?.getAttribute("data-entity") || null;
	referrer = document.referrer;
}

const log = (message: string) => console.info("[liwan]: " + message);
const ignore = (reason: string) => log("Ignoring event: " + reason);
const reject = (message: string) => {
	throw new Error("Failed to send event: " + message);
};

/**
 * Sends an event to the Liwan API.
 *
 * @param name The name of the event. Defaults to "pageview".
 * @param options Additional options for the event. See {@link EventOptions}.
 * @returns A promise that resolves with the status code of the response or void if the event was ignored.
 * @throws If {@link EventOptions.endpoint} is not provided in server-side environments.
 *
 * @example
 * ```ts
 * // Send a pageview event
 * await event("pageview", {
 *   url: "https://example.com",
 *   referrer: "https://google.com",
 *   endpoint: "https://liwan.example.com/api/event"
 * }).then(({ status }) => {
 *   console.log(`Event response: ${status}`);
 * });
 * ```
 */
export async function event(name = "pageview", options?: EventOptions): Promise<void> {
	const endpoint_url = options?.endpoint || endpoint;
	if (!endpoint_url) return reject("endpoint is required");
	if (typeof localStorage !== "undefined" && localStorage.getItem("disable-liwan")) return ignore("localStorage flag");
	if (/^localhost$|^127(?:\.\d+){0,2}\.\d+$|^\[::1?\]$/.test(location.hostname) || location.protocol === "file:")
		return ignore("localhost");

	const utm = new URLSearchParams(location.search);
	const response = await fetch(endpoint_url, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(<Payload>{
			name,
			entity_id: options?.entity || entity,
			referrer: options?.referrer || referrer,
			url: options?.url || location.origin + location.pathname,
			utm: {
				campaign: utm.get("utm_campaign"),
				content: utm.get("utm_content"),
				medium: utm.get("utm_medium"),
				source: utm.get("utm_source"),
				term: utm.get("utm_term"),
			},
		}),
	});

	if (!response.ok) {
		log("Failed to send event: " + response.statusText);
		reject(response.statusText);
	}
}

const trackPageviews = () => {
	window.__liwan_loaded = true;
	let lastPage: string | undefined;
	const page = () => {
		if (lastPage === location.pathname) return;
		lastPage = location.pathname;
		event("pageview");
	};

	if (window.navigation) {
		// best case scenario, we can listen for navigation events
		// sadly this is not available on firefox or safari
		window.navigation.addEventListener("navigate", () => page());
	} else {
		// duplicate navigation events don't matter
		// as we check if the page has changed above

		// try to intercept history.pushState, but it's not always possible
		window.history.pushState = new Proxy(window.history.pushState, {
			apply: (target, thisArg, argArray) => {
				target.apply(thisArg, argArray);
				page();
			},
		});

		// popstate is pretty buggy
		window.addEventListener("popstate", page);

		// astro copies history.pushState, so we need to listen for astro:page-load
		document.addEventListener("astro:page-load", () => page());
	}

	// initial pageview
	page();
};

if (!noWindow && !window.__liwan_loaded && scriptEl) {
	trackPageviews();
}
