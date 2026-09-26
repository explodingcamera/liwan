# Liwan Tracker

Tracking script for [Liwan](https://liwan.dev), an open-source analytics platform.

For server-side events, use [liwan-api](https://www.npmjs.com/package/liwan-api).

The `liwan-tracker` npm package is intended for current Liwan server releases. Its network protocol may change between Liwan versions. When it does, update the npm tracker alongside Liwan. If you use the tracker script served by Liwan itself, it already uses the matching internal tracker version.

## Usage

### Pageviews

When the script is loaded directly in the browser, it will automatically send pageview events to the API endpoint specified in the `data-api` attribute (`data-api` is optional and defaults to the domain name the script is loaded from).

```html
<script
  type="module"
  src="https://liwan.example.com/script.js"
  data-entity="example"
  data-api="https://liwan.example.com/api/event"
></script>
```

Pageviews send a best-effort exit signal when the page becomes hidden. Set `data-exit="false"` to disable exit tracking.

When using the npm package, call `trackPageviews()` to start automatic pageview tracking:

```ts
import { trackPageviews } from "liwan-tracker";

trackPageviews({
  endpoint: "https://liwan.example.com/api/event",
  entity: "example",
});
```

### Custom events

```ts
import { event } from "liwan-tracker";

await event("pageview", {
  url: "https://example.com",
  referrer: "https://google.com",
  endpoint: "https://liwan.example.com/api/event",
  entity: "example",
});
```

## API

```ts
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
   * Whether this event should be reported again when the page becomes hidden.
   *
   * Defaults to `true` for pageviews and `false` for other events. This option
   * is ignored in server-side environments.
   */
  exit?: boolean;
};

/**
 * Sends an event to the Liwan API.
 *
 * @param name The name of the event. Defaults to "pageview".
 * @param options Additional options for the event. See {@link EventOptions}.
 * @returns A promise that resolves when the event has been sent.
 * @throws If {@link EventOptions.endpoint} is not provided in server-side environments.
 */
export function event(
  name: string = "pageview",
  options?: EventOptions
): Promise<void>;

/**
 * Starts automatically tracking pageviews.
 *
 * Sends an initial pageview immediately and tracks subsequent client-side
 * navigations using the Navigation API when available, with `popstate` as a fallback.
 *
 * @param options Options passed to each pageview event.
 */
export const trackPageviews: (options?: EventOptions) => void;
```

## License

Licensed under the [Apache License, Version 2.0](https://github.com/explodingcamera/liwan/blob/main/LICENSE.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Liwan by you, as defined in the Apache-2.0 license, shall be licensed under Apache-2.0, without any additional terms or conditions.
