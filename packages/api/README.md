# liwan-api

JavaScript and TypeScript client for the [Liwan](https://liwan.dev) API.

```sh
npm install liwan-api
```

```ts
import { createClient } from "liwan-api";

const client = createClient({
  endpoint: "https://liwan.example.com",
  apiKey: process.env.LIWAN_API_KEY!,
});
```

See [examples/](examples/) for usage examples. For client-side tracking, use [liwan-tracker](https://www.npmjs.com/package/liwan-tracker).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/explodingcamera/liwan/blob/main/LICENSE.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Liwan by you, as defined in the Apache-2.0 license, shall be licensed under Apache-2.0, without any additional terms or conditions.
