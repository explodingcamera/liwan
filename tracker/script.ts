declare global {
	interface Window {
		__liwan_loaded?: boolean;
	}
}

type Payload = {
	name: string;
	url: string;
	referrer?: string;
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

if (typeof document !== "undefined") {
	scriptEl = document.currentScript as HTMLScriptElement;
	endpoint = scriptEl?.getAttribute("data-api") || (scriptEl && `${new URL(scriptEl.src).origin}/api/event`);
	entity = scriptEl?.getAttribute("data-entity") || null;
	referrer = document.referrer;
}

const LOCALHOST_REGEX = /^localhost$|^127(\.[0-9]+){0,2}\.[0-9]+$|^\[::1?\]$/;
const ignoreEvent = (reason: string) => console.info(`[liwan]: ignoring event: ${reason}`);

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
export async function event(name = "pageview", options?: EventOptions) {
	if (typeof window === "undefined" && !options?.endpoint)
		return Promise.reject(new Error("endpoint is required in server-side environments"));
	if (typeof localStorage !== "undefined" && localStorage.getItem("disable-liwan"))
		return ignoreEvent("localStorage flag");
	if (LOCALHOST_REGEX.test(location.hostname) || location.protocol === "file:") return ignoreEvent("localhost");
	if (!endpoint && !options?.endpoint) return ignoreEvent("no endpoint");

	// biome-ignore lint/style/noNonNullAssertion: we know that endpoint is not null
	return fetch((options?.endpoint || endpoint)!, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(<Payload>{
			entity_id: options?.entity || entity,
			name,
			referrer: options?.referrer || referrer,
			url: options?.url || `${location.origin}${location.pathname}`,
		}),
	}).then((response) => {
		if (!response.ok) console.error("[liwan]: failed to send event: ", response);
		return { status: response.status };
	});
}

const trackPageviews = () => {
	window.__liwan_loaded = true;
	let lastPage: string | undefined;
	const page = () => {
		if (lastPage === location.pathname) return;
		lastPage = location.pathname;
		event("pageview");
	};

	if (history.pushState) {
		const originalPushState = history.pushState;
		history.pushState = (...args: Parameters<typeof history.pushState>) => {
			originalPushState(...args);
			page();
		};
		window.addEventListener("popstate", page);
	}
	page();
};

if (typeof window !== "undefined" && !window.__liwan_loaded && scriptEl) {
	trackPageviews();
}
