# Liwan Tracker

Tracking script for [Liwan](https://liwan.dev), an open-source analytics platform.

Tracker versions follow the Liwan release needed for their latest features, with patch releases numbered independently. Older trackers still work with newer servers, but need an update to use new features.

For server-side events, use [@liwan.dev/api](https://www.npmjs.com/package/@liwan.dev/api).

## Install

```sh
# npm
npm install @liwan.dev/tracker

# Bun
bun add @liwan.dev/tracker

# pnpm
pnpm add @liwan.dev/tracker
```

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

When using the package, call `trackPageviews()` to start automatic pageview tracking:

```ts
import { trackPageviews } from "@liwan.dev/tracker";

trackPageviews({
  endpoint: "https://liwan.example.com/api/event",
  entity: "example",
});
```

### Custom events

Call `event` when an action happens. In the browser, the tracker uses the current page URL and referrer.

```ts
import { event } from "@liwan.dev/tracker";

await event("signup", {
  endpoint: "https://liwan.example.com/api/event",
  entity: "example",
  properties: { plan: "pro" },
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

export type EventProperties = Record<string, string | number | boolean | null | undefined>;

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
