# liwan-api

Rust client for the [Liwan](https://liwan.dev) API.

```sh
cargo add liwan-api --features reqwest,tokio
cargo add reqwest --no-default-features --features rustls
cargo add tokio --features macros,rt-multi-thread
```

```rust
use liwan_api::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(
        "https://liwan.example.com",
        std::env::var("LIWAN_API_KEY")?,
        reqwest::Client::new(),
    )?
    .build_tokio()?;

    client.event("docs", liwan_api::Event::pageview("https://example.com/docs"))?;
    client.shutdown().await?;
    Ok(())
}
```

See [examples/](examples/) and the [API documentation](https://docs.rs/liwan-api).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/explodingcamera/liwan/blob/main/LICENSE.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Liwan by you, as defined in the Apache-2.0 license, shall be licensed under Apache-2.0, without any additional terms or conditions.
