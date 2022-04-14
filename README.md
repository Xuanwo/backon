# backon

The opposite backoff implementation of the popular [backoff](https://docs.rs/backoff).

- Newer: developed by Rust edition 2021 and latest stable.
- Cleaner: Iterator based abstraction, easy to use, customization friendly.
- Easier: Trait based implementations, works like a native function provided by closures.

## Quick Start

```rust
use backon::Retryable;
use backon::ExponentialBackoff;
use anyhow::Result;

async fn fetch() -> Result<String> {
    Ok(reqwest::get("https://www.rust-lang.org").await?.text().await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let content = fetch.retry(ExponentialBackoff::default()).await?;
    println!("fetch succeeded: {}", contet);

    Ok(())
}
```
