# @liwan.dev/api

JavaScript and TypeScript client for the [Liwan](https://liwan.dev) API.

While the API is pre-1.0, its minor version tracks the Liwan 1.x release needed for its latest features. Patch releases are independent, and older clients keep working with newer servers but need an update for new features.

## Install

```sh
# npm
npm install @liwan.dev/api

# Bun
bun add @liwan.dev/api

# pnpm
pnpm add @liwan.dev/api
```

```ts
import { createClient } from "@liwan.dev/api";

const client = createClient({
  endpoint: "https://liwan.example.com",
  apiKey: process.env.LIWAN_API_KEY!,
});
```

See [examples/](examples/) for usage examples. For client-side tracking, use [@liwan.dev/tracker](https://www.npmjs.com/package/@liwan.dev/tracker).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/explodingcamera/liwan/blob/main/LICENSE.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Liwan by you, as defined in the Apache-2.0 license, shall be licensed under Apache-2.0, without any additional terms or conditions.
