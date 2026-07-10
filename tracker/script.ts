declare global {
	interface Window {
		__liwan_loaded?: boolean;
	}
}

type Payload = {
	name: string;
	entity_id?: string;
	url: string;
	referrer?: string;
	screen_width?: string;
	orientation?: string;
	// biome-ignore lint/suspicious/noExplicitAny: we want to allow any additional properties to be sent in the payload
} & Record<string, any>;

export type EventOptions = {
	/**
	 * The URL of the page where the event occurred.
	 *
	 * If not provided, the current page URL with only attribution query parameters preserved will be used.
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
	scriptEl =
		document.querySelector<HTMLScriptElement>(`script[src^="${import.meta.url}"]`) ??
		document.querySelector<HTMLScriptElement>("script:not([src])[data-api][data-entity]");

	endpoint =
		scriptEl?.getAttribute("data-api") || (scriptEl?.src && `${new URL(scriptEl.src).origin}/api/event`) || null;

	entity = scriptEl?.getAttribute("data-entity") || null;
	referrer = document.referrer;
}

const log = (message: string) => console.info(`[liwan]: ${message}`);
const ignore = (reason: string) => log(`Ignoring event: ${reason}`);
const reject = (message: string) => {
	throw new Error(`Failed to send event: ${message}`);
};

const ATTRIBUTION_QUERY_PARAMS = [
	"utm_campaign",
	"utm_content",
	"utm_medium",
	"utm_source",
	"utm_term",
	"campaign",
	"content",
	"medium",
	"source",
	"term",
	"ref",
	"referrer",
	"referer",
];

const sanitizeUrl = (value: string) => {
	const url = !noWindow ? new URL(value, location.href) : new URL(value);
	const params = new URLSearchParams();

	for (const [key, paramValue] of url.searchParams) {
		if (ATTRIBUTION_QUERY_PARAMS.includes(key)) {
			params.append(key, paramValue);
		}
	}

	url.search = params.toString();
	url.hash = "";
	return url.toString();
};

/**
 * Sends an event to the Liwan API.
 *
 * @param name The name of the event. Defaults to "pageview". Currencly, custom event names are not supported and will be treated as "pageview".
 * @param options Additional options for the event. See {@link EventOptions}.
 * @returns A promise that resolves when the event has been sent
 * @throws If {@link EventOptions.endpoint} is not provided in server-side environments.
 *
 * @example
 * ```ts
 * // Send a pageview event
 * await event("pageview", {
 *   url: "https://example.com",
 *   referrer: "https://google.com",
 *   endpoint: "https://liwan.example.com/api/event"
 * });
 * ```
 */
export async function event(name: string = "pageview", options?: EventOptions): Promise<void> {
	const endpoint_url = options?.endpoint || endpoint;
	if (!endpoint_url) return reject("endpoint is required");
	if (!noWindow && localStorage?.getItem("disable-liwan")) return ignore("localStorage flag");
	if (
		!noWindow &&
		(/^localhost$|^127(?:\.\d+){0,2}\.\d+$|^(?:\[::1\]|::1)$/.test(location.hostname) || location.protocol === "file:")
	)
		return ignore("localhost");

	const w = !noWindow ? window.screen?.width : undefined;

	// biome-ignore format: more readable this way
	const screen_width =
        w == null ? undefined :
        w < 480 ? "xs" :
        w < 768 ? "sm" :
        w < 1024 ? "md" :
        w < 1280 ? "lg" :
        w < 1536 ? "xl" :
        "2xl";

	const url = options?.url || (!noWindow ? location.href : null);
	if (!url) return reject("url is required");

	const response = await fetch(endpoint_url, {
		method: "POST",
		headers: { "Content-Type": "text/plain;charset=UTF-8" }, // we use text/plain to avoid preflight requests
		keepalive: true, // allow the request to be sent even if the page is being unloaded
		body: JSON.stringify(<Payload>{
			name,
			entity_id: options?.entity || entity,
			referrer: options?.referrer || referrer,
			url: sanitizeUrl(url),
			screen_width,
			orientation: noWindow
				? undefined
				: window.screen.orientation?.type.startsWith("portrait")
					? "portrait"
					: "landscape",
		}),
	});

	if (!response.ok) {
		reject(`${response.status} ${response.statusText}`.trim());
	}
}

/**
 * Starts automatically tracking pageviews.
 *
 * Sends an initial pageview immediately and tracks subsequent client-side
 * navigations using the Navigation API when available, with `popstate` as a fallback.
 *
 * Calling this function marks Liwan as loaded through `window.__liwan_loaded`.
 *
 * @param options Options passed to each pageview event.
 *
 * @example
 * ```ts
 * import { trackPageviews } from "@liwan/tracker";
 *
 * trackPageviews({
 *   endpoint: "https://analytics.example.com/api/event",
 *   entity: "example",
 * });
 * ```
 */
export const trackPageviews = (options?: EventOptions) => {
	window.__liwan_loaded = true;
	let lastPage: string | undefined;

	const page = () => {
		if (lastPage === location.pathname) return;
		lastPage = location.pathname;

		void event("pageview", options).catch((error) => log(error instanceof Error ? error.message : String(error)));
	};

	if (window.navigation) {
		// baseline since Jan 2026
		window.navigation.addEventListener("currententrychange", () => page());
	} else {
		// not the best fallback but most browsers support the new Navigation API
		window.addEventListener("popstate", () => page());
	}

	// initial pageview
	page();
};

if (!noWindow && !window.__liwan_loaded && scriptEl) {
	trackPageviews();
}
