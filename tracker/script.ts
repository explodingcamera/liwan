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
	overrideUrl?: string;
	overrideReferrer?: string;
};

const LOCALHOST_REGEX = /^localhost$|^127(\.[0-9]+){0,2}\.[0-9]+$|^\[::1?\]$/;
const scriptEl = document?.currentScript as HTMLScriptElement;
const endpoint = scriptEl?.getAttribute("data-api") || (scriptEl && `${new URL(scriptEl.src).origin}/api/event`);
const ignoreEvent = (reason: string) => console.info(`[liwan]: ignoring event: ${reason}`);

export async function event(name: string, options?: EventOptions) {
	if (localStorage.getItem("disable-analytics")) return ignoreEvent("localStorage flag");
	if (LOCALHOST_REGEX.test(location.hostname) || location.protocol === "file:") return ignoreEvent("localhost");

	return fetch(endpoint, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(<Payload>{
			name,
			referrer: options?.overrideReferrer || document.referrer,
			url: options?.overrideUrl || `${location.origin}${location.pathname}`,
		}),
	}).then((response) => {
		if (!response.ok) console.error("[liwan]: failed to send event: ", response);
		return { status: response.status };
	});
}

const trackPageviews = () => {
	if (window.__liwan_loaded) return;
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

if (!window.__liwan_loaded && scriptEl) {
	trackPageviews();
} else {
	console.info("[liwan]: already loaded, skipping pageview tracking");
}
