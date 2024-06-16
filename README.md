# liwan - Lightweight Analytics

> [!WARNING]  
> This project is still in the early stages of development and not ready to be used.

- Lightweight and privacy-friendly
- Easy to deploy and host on your own server
- Doesn't collect IP addresses or other personal information
- Single, statically linked binary with no external dependencies (powered by DuckDB)
- Extremely permissive license (Apache-2.0 OR MIT)
- No tracking across multiple sites
- No Cookies/LocalStorage/...
- Tracking script is less than 0.5 KB

## Usage

Run the binary and point your browser to `http://localhost:8080`.

```sh
$ curl -JLO <url>
$ chmod +x liwan
$ ./liwan
```

## Configuration

Users and sites are configured in the liwan.config.toml file (The web UI is read-only). When you first run the binary, a config file will be created with all the information you need to get started.

To generate a password hash for a new user, run the following command:

```sh
$ ./liwan hash-password <password>
```

## Privacy

### What data is collected?

- The URL of the page that the tracker is embedded in (excluding query parameters and fragments)
- The Referrer URL if available (the page that the user came from)
- Browser, OS, mobile or desktop, and locale (e.g. Chrome, Windows, Desktop, en-US)
- Anonymized visitor ID (generated with `sha3-256(ip, user_agent, daily_salt, entity_id)[0:16]`)
- The time the event occurred
- Country or City (optional)
- Custom event data (optional)

IP addresses and user agent are discarded immediately after generating the visitor ID and never stored on disk or logged. Additionally, the daily salt is rotated every 24 hours to ensure that the visitor ID cannot be linked across multiple days. To reduce the risk of fingerprinting, no version information or screen resolution is collected.

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT license](./LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in TinyWasm by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

The data contained in `src/regexes.yaml` is Copyright 2009 Google Inc. and available under the Apache License, Version 2.0.
