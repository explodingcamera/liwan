declare global {
    interface Window {
        __liwan_loaded?: boolean;
    }
}
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
export declare function event(name?: string, options?: EventOptions): Promise<void>;
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
export declare const trackPageviews: (options?: EventOptions) => void;
