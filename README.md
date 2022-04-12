# backon

The opposite backoff implementation of the popular [backoff](https://docs.rs/backoff).

- Newer: developed by Rust edition 2021 and latest stable.
- Cleaner: Iterator based abstraction, easy to use, customization friendly.
- Smaller: Focused on backoff implementation, no need to play with runtime specific features.

## Quick Start

```rust
use backon::ExponentialBackoff;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    for delay in ExponentialBackoff::default() {
        let x = reqwest::get("https://www.rust-lang.org").await?.text().await;
        match x {
            Ok(v) => {
                println!("Successfully fetched");
                break;
            },
            Err(_) => {
                tokio::time::sleep(delay).await;
                continue
            }
        };
    }

    Ok(())
}
```
