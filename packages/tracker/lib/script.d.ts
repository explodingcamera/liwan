//#region src/script.d.ts
declare global {
  interface Window {
    __liwan_loaded?: boolean;
  }
  /** Default properties included with subsequent events. Set before loading the script to include the first pageview. */
  var liwanProperties: EventProperties | undefined;
  /** Transform a normalized event payload before it is sent. Not called for exit signals. */
  var liwanTransformEvent: ((payload: EventPayload) => EventPayload) | undefined;
}
/** Normalized event sent to the API. The event transformer can edit this before it is sent. */
export type EventPayload = {
  /** Event name, such as `pageview` or `signup`. */
  name: string;
  /** Entity associated with the event. */
  entity_id?: string;
  /** Page URL with unrecognized query parameters and the fragment removed. */
  url: string;
  /** Referrer supplied by the caller or the document. */
  referrer?: string;
  /** Screen-width bucket, when available in the browser. */
  screen_width?: string;
  /** Screen orientation, when available in the browser. */
  orientation?: string;
  /** Global defaults merged with event-specific properties. */
  properties?: EventProperties;
  /** Marks a pageview exit signal. */
  exit?: boolean;
};
export type EventProperties = Record<string, string | number | boolean | null | undefined>;
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
  /**
   * Whether to send a pageview exit signal when the page becomes hidden.
   *
   * Defaults to `true`. Ignored for custom events and in server-side environments.
   */
  exit?: boolean;
  /**
   * Custom properties to send with the event.
   */
  properties?: EventProperties;
};
/** Set default properties for subsequent events. Pass `null` to remove a property. */
export declare function setProperties(properties: EventProperties): void;
/**
 * Sends an event to the Liwan API.
 *
 * @param name The name of the event. Defaults to "pageview".
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
 * Subsequent calls are ignored.
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
//#endregion