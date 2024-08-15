# liwan - Lightweight Analytics

> [!WARNING]
> This project is still in the early stages of development and not ready to be used.

- Lightweight and privacy-friendly
- Easy to deploy and host on your own server
- Doesn't collect IP addresses or other personal information
- Single, statically linked binary with no external dependencies (powered by DuckDB)
- Permissive license (Apache-2.0 OR MIT)
- No tracking across multiple sites
- No Cookies/LocalStorage/...
- Tracking script is less than 1 KB

## Usage

Run the binary and point your browser to `http://localhost:8080`.

```sh
$ curl -JLO <url>
$ tar -xzf liwan-*.tar.xz
$ chmod +x liwan
$ ./liwan
```

## Privacy

### What data is collected?

- The URL of the page that the tracker is embedded in (excluding query parameters and fragments)
- The Referrer URL if available (the page that the user came from)
- Browser, OS (e.g. Chrome, Windows, Desktop)
- A anonymized visitor ID (generated with `sha3-256(ip, user_agent, daily_salt, entity_id)[0:16]`)
- The time the event occurred
- Country or City (optional)
- Custom event data (optional)

IP addresses and user agent are discarded immediately after generating the visitor ID and never stored on disk or logged. Additionally, the daily salt is rotated every 24 hours to ensure that the visitor ID cannot be linked across multiple days. To reduce the risk of fingerprinting, no version information or screen resolution is collected and the entity ID is included in the hash to prevent tracking across multiple sites (e.g. if the same user visits two different sites that use the same instance of liwan, both will be counted as separate visitors).

## License

Unless otherwise noted, code is available under the terms of the GNU AGPLv3 license. See [LICENSE](LICENSE.md) for more information.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Liwan by you, as defined in the Apache-2.0 license, shall additionally be licensed under the terms of both the Apache License, Version 2.0 and the MIT license without any additional terms or conditions. This dual licensing is intended to allow possible future re-licensing under a more permissive license and monitization of the project. If you do not agree with this dual licensing, you should not contribute to the project (which is fine! you can still use and modify it under the terms of the AGPLv3).

### Third-party licenses

- `data/ua_regexes.yaml` is based on data from [ua-parser/uap-core](https://github.com/ua-parser/uap-core/blob/master/regexes.yaml) (Copyright 2009 Google Inc. and available under the Apache License, Version 2.0)
- `data/spammers.txt` is in the public domain (see [matomo-org/referrer-spam-list](https://github.com/matomo-org/referrer-spam-list))
- `data/socials.txt` is based on [matomo-org/searchengine-and-social-list](https://github.com/matomo-org/searchengine-and-social-list) (available under the CC0 1.0 Universal Public Domain Dedication)
- `data/geo.json` is based on data from [Natural Earth](https://naturalearthdata.com/) (which itself is in the public domain)
- The code in `tracker/` is available as a separate library under the terms of the MIT license (see [LICENSE](tracker/LICENSE.md))
