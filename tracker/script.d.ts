declare module "script" {
    global {
        interface Window {
            __liwan_loaded?: boolean;
        }
    }
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
    export function event(name?: string, options?: EventOptions): Promise<void | {
        status: number;
    }>;
}
